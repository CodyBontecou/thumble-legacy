package com.thumble.android

import android.graphics.Bitmap
import android.graphics.Color
import android.os.Handler
import android.os.Looper
import android.view.PixelCopy
import androidx.test.core.app.ActivityScenario
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import java.util.UUID
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger

@RunWith(AndroidJUnit4::class)
class AboveLegendHardwareDeviceTest {
    @Test fun aboveLegendSurvivesHardwareChildDisplayListClipping() {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val previous = ClientDependencies.sessionStore
        ClientDependencies.sessionStore = { MemorySessionStore() }
        try { ActivityScenario.launch(MainActivity::class.java).use { scenario ->
            lateinit var surface: ControllerSurface
            val drawn = CountDownLatch(1)
            scenario.onActivity { activity ->
                val content = JSONObject().put("legend", "A").put("fontSize", 12).put("labelPlacement", "above").put("labelPadding", 2)
                val style = JSONObject().put("centerX", .5).put("centerY", .5).put("widthScale", .75).put("heightScale", .75)
                    .put("visualStyle", JSONObject().put("normal", JSONObject().put("foregroundColor", rgbaJSON(Color.MAGENTA)).put("content", content)))
                val element = JSONObject().put("id", UUID.randomUUID().toString()).put("kind", "button").put("label", "A").put("layout", style)
                val profile = ControllerProfile.parse(JSONObject().put("id", UUID.randomUUID().toString()).put("customization", JSONObject().put("elements", JSONArray().put(element))))
                surface = ControllerSurface(activity)
                surface.show(profile, 1, false, false)
                activity.setContentView(surface)
                surface.postOnAnimation { surface.postOnAnimation { drawn.countDown() } }
            }
            assertTrue(drawn.await(5, TimeUnit.SECONDS))
            instrumentation.waitForIdleSync()
            val copied = CountDownLatch(1); val result = AtomicInteger(-1)
            lateinit var bitmap: Bitmap
            val origin = IntArray(2); var cw = 0; var margin = 0
            scenario.onActivity { activity ->
                val child = surface.getChildAt(0); child.getLocationInWindow(origin); cw = child.width
                margin = (24 * surface.resources.displayMetrics.density).toInt()
                bitmap = Bitmap.createBitmap(activity.window.decorView.width, activity.window.decorView.height, Bitmap.Config.ARGB_8888)
                PixelCopy.request(activity.window, bitmap, { code -> result.set(code); copied.countDown() }, Handler(Looper.getMainLooper()))
            }
            assertTrue(copied.await(5, TimeUnit.SECONDS)); assertEquals(PixelCopy.SUCCESS, result.get())
            var ink = 0
            for (y in (origin[1] - margin).coerceAtLeast(0) until origin[1]) for (x in origin[0].coerceAtLeast(0) until (origin[0] + cw).coerceAtMost(bitmap.width)) {
                val p = bitmap.getPixel(x, y)
                if (Color.red(p) > 150 && Color.blue(p) > 150 && Color.green(p) < 100) ink++
            }
            bitmap.recycle()
            assertTrue("Hardware-rendered above legend must survive child display-list clipping", ink > 3)
        } } finally { ClientDependencies.sessionStore = previous }
    }
}
