package com.thumble.rewrite.input

import java.io.Closeable
import java.io.EOFException
import java.net.InetSocketAddress
import java.net.SocketException
import java.net.SocketTimeoutException
import java.util.ArrayDeque
import javax.net.ssl.SSLException

internal interface NativeFrameIO : Closeable {
    val failureCategory: NativeTLSExceptionCategory?
        get() = null

    val operationFailure: NativeTLSOperationFailure?
        get() = null

    fun send(frame: ControlTLSFrame)

    fun receive(): ControlTLSFrame
}

/** Fixed diagnostic values only: exception text and transport bytes never leave the owner. */
enum class NativeTLSCloseCause {
    EXPLICIT_CLOSE,
    CONNECT_FAILED,
    WRITE_FAILED,
    READ_FAILED,
    INVALID_INPUT,
    CONTROL_QUEUE_OVERFLOW,
    INPUT_QUEUE_OVERFLOW,
    RECEIVED_QUEUE_OVERFLOW,
    INVALID_DIRECTION,
}

enum class NativeTLSExceptionCategory {
    NONE,
    TIMEOUT,
    EOF,
    SOCKET,
    TLS,
    REJECTED,
    OTHER,
}

enum class NativeTLSOperationDirection {
    SEND,
    RECEIVE,
}

enum class NativeTLSOperationFailureKind {
    DEADLINE,
    IO,
}

data class NativeTLSOperationFailure(
    val direction: NativeTLSOperationDirection,
    val kind: NativeTLSOperationFailureKind,
    val category: NativeTLSExceptionCategory,
)

data class NativeTLSControlActivity(
    val queued: Long,
    val writeStarted: Long,
    val writeCompleted: Long,
    val received: Long,
    val queuedAgeMs: Long?,
    val writeStartedAgeMs: Long?,
    val writeCompletedAgeMs: Long?,
    val receivedAgeMs: Long?,
    val queueDepth: Int,
)

data class NativeTLSTerminalDiagnostic(
    val cause: NativeTLSCloseCause,
    val category: NativeTLSExceptionCategory,
    val operation: NativeTLSOperationFailure? = null,
)

internal fun tlsExceptionCategory(error: Exception): NativeTLSExceptionCategory =
    when (error) {
        is SocketTimeoutException -> NativeTLSExceptionCategory.TIMEOUT
        is EOFException -> NativeTLSExceptionCategory.EOF
        is SSLException -> NativeTLSExceptionCategory.TLS
        is SocketException -> NativeTLSExceptionCategory.SOCKET
        is ControlTLSRejected -> NativeTLSExceptionCategory.REJECTED
        else -> NativeTLSExceptionCategory.OTHER
    }

/**
 * Bounded native delivery owner, independent of React and the input/UI sampling thread. Call
 * offerInput directly from InputPacketChannel's synchronous sink. It never waits for socket I/O. A
 * ready TLS transport still needs authenticated enrollment/configuration before input admission.
 * close is terminal; a new connection and fresh packet grant are required after any failure.
 */
class NativeTLSDelivery private constructor(initial: NativeFrameIO?, connect: () -> NativeFrameIO) :
    Closeable {
    internal constructor(connect: () -> NativeFrameIO) : this(null, connect)

    internal constructor(raw: NativeFrameIO) : this(raw, { raw })

    enum class State {
        CONNECTING,
        READY,
        CLOSED,
    }

    private val gate = Object()
    private var current = if (initial == null) State.CONNECTING else State.READY
    private var io: NativeFrameIO? = initial
    private val controls = ArrayDeque<ControlTLSFrame>()
    private val received = ArrayDeque<ControlTLSFrame>()
    private var latestInput: ControlTLSFrame? = null
    private var terminal: NativeTLSTerminalDiagnostic? = null
    private var queued = 0L
    private var writeStarted = 0L
    private var writeCompleted = 0L
    private var receivedCount = 0L
    private var queuedAt: Long? = null
    private var writeStartedAt: Long? = null
    private var writeCompletedAt: Long? = null
    private var receivedAt: Long? = null
    private var activity: NativeTLSControlActivity? = null
    val terminalActivity: NativeTLSControlActivity?
        get() = synchronized(gate) { activity }

    private fun count(value: Long) = minOf(1_048_576L, value + 1)

    private fun age(now: Long, then: Long?): Long? = then?.let {
        ((now - it).coerceAtLeast(0) / 1_000_000).coerceAtMost(86_400_000)
    }

    val terminalDiagnostic: NativeTLSTerminalDiagnostic?
        get() = synchronized(gate) { terminal }

    val state: State
        get() = synchronized(gate) { current }

    init {
        Thread({ writeLoop { initial ?: connect() } }, "thumble-native-tls-writer")
            .apply { isDaemon = true }
            .start()
    }

    fun offerInput(packet: ByteArray): Boolean {
        val frame =
            try {
                ControlTLSFrame(ControlTLSFrame.Kind.INPUT, packet)
            } catch (error: Exception) {
                terminate(NativeTLSCloseCause.INVALID_INPUT, tlsExceptionCategory(error))
                return false
            }
        synchronized(gate) {
            if (current != State.READY) return false
            if (!frame.startsInputGrant) {
                latestInput = frame
                gate.notifyAll()
                return true
            }
            if (controls.size < 8) {
                // Preserve packet zero ahead of following status/control barriers.
                latestInput = null
                controls.addLast(frame)
                gate.notifyAll()
                return true
            }
        }
        terminate(NativeTLSCloseCause.INPUT_QUEUE_OVERFLOW, NativeTLSExceptionCategory.NONE)
        return false
    }

    fun offerControl(payload: ByteArray): Boolean {
        val frame =
            try {
                ControlTLSFrame(ControlTLSFrame.Kind.CONTROL, payload)
            } catch (error: Exception) {
                terminate(NativeTLSCloseCause.INVALID_INPUT, tlsExceptionCategory(error))
                return false
            }
        synchronized(gate) {
            if (current != State.READY) return false
            if (controls.size < 8) {
                queued = count(queued)
                queuedAt = System.nanoTime()
                controls.addLast(frame)
                gate.notifyAll()
                return true
            }
        }
        terminate(NativeTLSCloseCause.CONTROL_QUEUE_OVERFLOW, NativeTLSExceptionCategory.NONE)
        return false
    }

    /** Pull from the native control owner; never expose grant payloads to a React sample queue. */
    fun pollControl(): ControlTLSFrame? =
        synchronized(gate) {
            if (current == State.READY) received.pollFirst() else null
        }

    private fun writeLoop(connect: () -> NativeFrameIO) {
        var cause = NativeTLSCloseCause.CONNECT_FAILED
        var attempted: NativeFrameIO? = null
        try {
            val transport = connect()
            attempted = transport
            val accepted =
                synchronized(gate) {
                    if (current == State.CLOSED) false
                    else {
                        io = transport
                        current = State.READY
                        true
                    }
                }
            if (!accepted) {
                transport.close()
                return
            }
            cause = NativeTLSCloseCause.WRITE_FAILED
            Thread({ readLoop(transport) }, "thumble-native-tls-reader")
                .apply { isDaemon = true }
                .start()
            while (true) {
                val frame =
                    synchronized(gate) {
                        while (
                            current == State.READY && controls.isEmpty() && latestInput == null
                        ) gate.wait()
                        if (current != State.READY) return
                        if (controls.isNotEmpty()) controls.removeFirst()
                        else latestInput!!.also { latestInput = null }
                    }
                if (frame.kind == ControlTLSFrame.Kind.CONTROL)
                    synchronized(gate) {
                        writeStarted = count(writeStarted)
                        writeStartedAt = System.nanoTime()
                    }
                transport.send(frame)
                if (frame.kind == ControlTLSFrame.Kind.CONTROL)
                    synchronized(gate) {
                        writeCompleted = count(writeCompleted)
                        writeCompletedAt = System.nanoTime()
                    }
            }
        } catch (error: Exception) {
            terminate(cause, attempted?.failureCategory ?: tlsExceptionCategory(error))
        }
    }

    private fun readLoop(transport: NativeFrameIO) {
        var cause = NativeTLSCloseCause.READ_FAILED
        try {
            while (state == State.READY) {
                val frame = transport.receive()
                synchronized(gate) {
                    if (current != State.READY) return
                    if (frame.kind != ControlTLSFrame.Kind.CONTROL) {
                        cause = NativeTLSCloseCause.INVALID_DIRECTION
                        throw ControlTLSRejected()
                    }
                    if (received.size >= 8) {
                        cause = NativeTLSCloseCause.RECEIVED_QUEUE_OVERFLOW
                        throw ControlTLSRejected()
                    }
                    receivedCount = count(receivedCount)
                    receivedAt = System.nanoTime()
                    received.addLast(frame)
                }
            }
        } catch (error: Exception) {
            terminate(cause, transport.failureCategory ?: tlsExceptionCategory(error))
        }
    }

    override fun close() =
        terminate(NativeTLSCloseCause.EXPLICIT_CLOSE, NativeTLSExceptionCategory.NONE)

    private fun terminate(cause: NativeTLSCloseCause, category: NativeTLSExceptionCategory) {
        val transport =
            synchronized(gate) {
                if (current == State.CLOSED) return
                val now = System.nanoTime()
                activity =
                    NativeTLSControlActivity(
                        queued,
                        writeStarted,
                        writeCompleted,
                        receivedCount,
                        age(now, queuedAt),
                        age(now, writeStartedAt),
                        age(now, writeCompletedAt),
                        age(now, receivedAt),
                        controls.size,
                    )
                terminal = NativeTLSTerminalDiagnostic(cause, category, io?.operationFailure)
                current = State.CLOSED
                controls.clear()
                received.clear()
                latestInput = null
                gate.notifyAll()
                io.also { io = null }
            }
        // The UI/input caller must never wait for platform TLS shutdown or an in-flight socket
        // operation.
        // The terminal transition above permits at most one closer for this connection.
        if (transport != null)
            Thread(
                    {
                        try {
                            transport.close()
                        } catch (_: Exception) {}
                    },
                    "thumble-native-tls-close",
                )
                .apply { isDaemon = true }
                .start()
    }

    companion object {
        fun connect(address: InetSocketAddress, certificatePin: String): NativeTLSDelivery =
            NativeTLSDelivery {
                val client = ControlTLSClient.connect(address, certificatePin)
                client.nativeFrames()
            }
    }
}
