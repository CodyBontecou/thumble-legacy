package com.thumble.rewrite.qa

import android.content.Intent
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.uiautomator.By
import androidx.test.uiautomator.UiDevice
import androidx.test.uiautomator.Until
import org.junit.Assume.assumeTrue
import org.junit.Test
import org.junit.runner.RunWith

/** Actual default React route/native factory, with existing persisted synthetic phone trust. */
@RunWith(AndroidJUnit4::class)
class ReconnectRecoveryAppTest {
    @Test fun actualDefaultRouteReportsRecovery() {
        val args = InstrumentationRegistry.getArguments()
        assumeTrue("explicit recording recovery gate", args.getString("thumbleReconnectRecovery") == "1")
        val expected = args.getString("thumbleExpectedAdmission")
        check(expected == "ready" || expected == "reactivation-required")
        val target = InstrumentationRegistry.getInstrumentation().targetContext
        check(target.packageName == "com.codybontecou.thumble.rewrite.qa")
        val device = UiDevice.getInstance(InstrumentationRegistry.getInstrumentation())
        target.startActivity(checkNotNull(target.packageManager.getLaunchIntentForPackage(target.packageName))
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
        checkNotNull(device.wait(Until.findObject(By.res("home-connect")), 15000)).click()
        check(device.wait(Until.hasObject(By.text("Test your connection")), 10000))
        checkNotNull(device.findObject(By.res("android:id/button3"))).click()
        val message = if (expected == "ready") "Connected to receiver" else
            "Your saved profile changed. Ask your agent to reactivate it on your Mac, then close and reconnect. Your phone is still paired."
        try {
            check(device.wait(Until.hasObject(By.text(message)), 15000)) { "reconnect_expected_admission_missing" }
            check(!device.hasObject(By.text("Ask your agent to start the receiver and open pairing, then try again.")))
            if (expected == "reactivation-required") check(!device.hasObject(By.text("Connected to receiver")))
            // Optional observation window is test-only; permits an independent Argent capture.
            if (args.getString("thumbleObserveAdmission") == "1") Thread.sleep(10000)
        } finally {
            device.findObject(By.res("connection-done"))?.click()
        }
    }
}
