package com.thumble.rewrite.input

import java.io.EOFException
import java.net.SocketException
import java.net.SocketTimeoutException
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import javax.net.ssl.SSLException
import org.junit.Assert.*
import org.junit.Test

class NativeTLSDiagnosticTest {
    private fun until(condition: () -> Boolean) {
        val end = System.nanoTime() + TimeUnit.SECONDS.toNanos(3)
        while (System.nanoTime() < end) {
            if (condition()) return
            Thread.sleep(1)
        }
        fail("diagnostic_condition_timeout")
    }

    @Test
    fun terminalReadClassifiesOnlyFixedCategoriesAndCleanupCannotReplaceFirstCause() {
        val errors =
            listOf(
                SocketTimeoutException() to NativeTLSExceptionCategory.TIMEOUT,
                EOFException() to NativeTLSExceptionCategory.EOF,
                SocketException() to NativeTLSExceptionCategory.SOCKET,
                SSLException("") to NativeTLSExceptionCategory.TLS,
                ControlTLSRejected() to NativeTLSExceptionCategory.REJECTED,
                IllegalStateException() to NativeTLSExceptionCategory.OTHER,
            )
        for ((error, category) in errors) {
            val delivery =
                NativeTLSDelivery(
                    object : NativeFrameIO {
                        override fun send(frame: ControlTLSFrame) {}

                        override fun receive(): ControlTLSFrame = throw error

                        override fun close() {}
                    }
                )
            until { delivery.state == NativeTLSDelivery.State.CLOSED }
            val expected = NativeTLSTerminalDiagnostic(NativeTLSCloseCause.READ_FAILED, category)
            assertEquals(expected, delivery.terminalDiagnostic)
            repeat(3) { delivery.close() }
            assertEquals(expected, delivery.terminalDiagnostic)
            assertNull(delivery.pollControl())
            assertFalse(delivery.offerControl(byteArrayOf(1)))
        }
    }

    @Test
    fun explicitCloseWinsBeforeInterruptedReaderAndWriterErrors() {
        val entered = CountDownLatch(1)
        val release = CountDownLatch(1)
        val delivery =
            NativeTLSDelivery(
                object : NativeFrameIO {
                    override fun send(frame: ControlTLSFrame) {}

                    override fun receive(): ControlTLSFrame {
                        entered.countDown()
                        check(release.await(3, TimeUnit.SECONDS))
                        throw SocketException()
                    }

                    override fun close() {
                        release.countDown()
                    }
                }
            )
        assertTrue(entered.await(3, TimeUnit.SECONDS))
        delivery.close()
        val expected =
            NativeTLSTerminalDiagnostic(
                NativeTLSCloseCause.EXPLICIT_CLOSE,
                NativeTLSExceptionCategory.NONE,
            )
        assertEquals(expected, delivery.terminalDiagnostic)
        assertTrue(release.await(3, TimeUnit.SECONDS))
        assertEquals(expected, delivery.terminalDiagnostic)
    }

    @Test
    fun connectAndWriteFailuresRemainDifferentFromReaderAndExplicitClose() {
        val failed = NativeTLSDelivery { throw SocketException() }
        until { failed.state == NativeTLSDelivery.State.CLOSED }
        assertEquals(NativeTLSCloseCause.CONNECT_FAILED, failed.terminalDiagnostic?.cause)
        val entered = CountDownLatch(1)
        val release = CountDownLatch(1)
        val delivery =
            NativeTLSDelivery(
                object : NativeFrameIO {
                    override fun send(frame: ControlTLSFrame) {
                        throw SocketException()
                    }

                    override fun receive(): ControlTLSFrame {
                        entered.countDown()
                        check(release.await(3, TimeUnit.SECONDS))
                        throw SocketException()
                    }

                    override fun close() {
                        release.countDown()
                    }
                }
            )
        assertTrue(entered.await(3, TimeUnit.SECONDS))
        assertTrue(delivery.offerControl(byteArrayOf(1)))
        until { delivery.state == NativeTLSDelivery.State.CLOSED }
        assertEquals(
            NativeTLSTerminalDiagnostic(
                NativeTLSCloseCause.WRITE_FAILED,
                NativeTLSExceptionCategory.SOCKET,
            ),
            delivery.terminalDiagnostic,
        )
    }

    @Test
    fun malformedDirectionHasASeparateFixedCause() {
        val delivery =
            NativeTLSDelivery(
                object : NativeFrameIO {
                    override fun send(frame: ControlTLSFrame) {}

                    override fun receive() =
                        ControlTLSFrame(ControlTLSFrame.Kind.INPUT, ByteArray(108))

                    override fun close() {}
                }
            )
        until { delivery.state == NativeTLSDelivery.State.CLOSED }
        assertEquals(
            NativeTLSTerminalDiagnostic(
                NativeTLSCloseCause.INVALID_DIRECTION,
                NativeTLSExceptionCategory.REJECTED,
            ),
            delivery.terminalDiagnostic,
        )
    }

    @Test
    fun invalidClockRejectsBeforeTransportAndCleanupPreservesTheCheckpointReason() {
        var connected = false
        val client =
            PhoneControlClient(
                TrustedHostEndpoint("127.0.0.1", 1234, "11".repeat(32), "authority-a"),
                {
                    connected = true
                    throw IllegalStateException()
                },
                { _, _ -> null },
                { -1L },
            )
        try {
            client.authenticate()
            fail("invalid_clock_accepted")
        } catch (_: PhoneControlRejected) {}
        assertFalse(connected)
        assertEquals(PhoneCheckpointFailure.INVALID_TIME, client.checkpointFailure)
        client.close()
        try {
            client.status()
            fail("closed_status_accepted")
        } catch (_: PhoneControlRejected) {}
        assertEquals(PhoneCheckpointFailure.INVALID_TIME, client.checkpointFailure)
    }
}
