package com.thumble.android

import org.json.JSONObject
import org.junit.Test

class SkinModelCodecTest {
    private fun oracle()=SkinModelOracle {
        javaClass.classLoader!!.getResourceAsStream("skin-resolution/v1/native-model-oracle.json")!!.use { JSONObject(it.bufferedReader().readText()) }
    }
    @Test fun validModelsMatchNativeNormalizationAndPreserveInput() = oracle().validModelsMatchNativeNormalizationAndPreserveInput()
    @Test fun publishedPackagesMatchNativeSkinModels() {
        val native=javaClass.classLoader!!.getResourceAsStream("skin-resolution/v1/native-oracle.json")!!.use { JSONObject(it.bufferedReader().readText()) }
        oracle().publishedPackagesMatchNativeSkinModels(native) { name -> javaClass.classLoader!!.getResourceAsStream(name)!!.use { it.readBytes() } }
    }
    @Test fun malformedModelsMatchNativeRejections() = oracle().malformedModelsMatchNativeRejections()
    @Test fun independentIconPresentationNormalizesAndRejectsUnknownPlacement() {
        val normalized = SkinModelCodec.content(JSONObject().put("iconPlacement", "center").put("iconScale", 20))
        org.junit.Assert.assertEquals("center", normalized.getString("iconPlacement"))
        org.junit.Assert.assertEquals(3.0, normalized.getDouble("iconScale"), 0.0)
        org.junit.Assert.assertFalse(normalized.has("icon"))
        org.junit.Assert.assertThrows(SkinPackageException::class.java) {
            SkinModelCodec.content(JSONObject().put("iconPlacement", "unknown"))
        }
    }

    @Test fun optionalCircleIconMaskPreservesDefaultAndRejectsUnknownShapes() {
        org.junit.Assert.assertFalse(SkinModelCodec.content(JSONObject()).has("iconMask"))
        for (mask in listOf("none", "circle")) {
            org.junit.Assert.assertEquals(mask, SkinModelCodec.content(JSONObject().put("iconMask", mask)).getString("iconMask"))
        }
        org.junit.Assert.assertThrows(SkinPackageException::class.java) {
            SkinModelCodec.content(JSONObject().put("iconMask", "square-ish"))
        }
    }

}
