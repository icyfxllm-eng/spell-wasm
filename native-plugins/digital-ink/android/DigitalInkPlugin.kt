// DigitalInkPlugin — Capacitor bridge to ML Kit Digital Ink Recognition (Android).
//
// STAGED, NOT YET IN THE BUILD. iOS is the launch target (spec §4); this keeps
// the interface identical for a future Android build. Add to the app source set
// + the ML Kit gradle dependency, and register in MainActivity (see README.md).
//
// Signatures are from the Android ML Kit Digital Ink API; RE-VERIFY against the
// current docs at integration time (VERIFY-FIRST rule). Unlike iOS, Android
// candidates DO expose a numeric score (candidate.score).

package net.spellgame.app

import com.getcapacitor.JSObject
import com.getcapacitor.Plugin
import com.getcapacitor.PluginCall
import com.getcapacitor.PluginMethod
import com.getcapacitor.annotation.CapacitorPlugin
import com.google.mlkit.common.model.DownloadConditions
import com.google.mlkit.common.model.RemoteModelManager
import com.google.mlkit.vision.digitalink.Ink
import com.google.mlkit.vision.digitalink.DigitalInkRecognition
import com.google.mlkit.vision.digitalink.DigitalInkRecognitionModel
import com.google.mlkit.vision.digitalink.DigitalInkRecognitionModelIdentifier
import com.google.mlkit.vision.digitalink.DigitalInkRecognizerOptions
import com.google.mlkit.vision.digitalink.RecognitionContext
import com.google.mlkit.vision.digitalink.WritingArea

@CapacitorPlugin(name = "DigitalInk")
class DigitalInkPlugin : Plugin() {

    private val remoteModelManager = RemoteModelManager.getInstance()

    private fun modelFor(tag: String): DigitalInkRecognitionModel? {
        val identifier = try {
            DigitalInkRecognitionModelIdentifier.fromLanguageTag(tag)
        } catch (e: Exception) {
            null
        } ?: return null
        return DigitalInkRecognitionModel.builder(identifier).build()
    }

    @PluginMethod
    fun downloadModel(call: PluginCall) {
        val model = modelFor(call.getString("languageTag") ?: "")
            ?: return call.reject("unknown or unsupported languageTag", "BAD_TAG")
        remoteModelManager.isModelDownloaded(model).addOnSuccessListener { already ->
            if (already) {
                call.resolve(JSObject().put("status", "already"))
            } else {
                remoteModelManager
                    .download(model, DownloadConditions.Builder().build())
                    .addOnSuccessListener { call.resolve(JSObject().put("status", "downloaded")) }
                    .addOnFailureListener { e -> call.reject("download failed: ${e.message}", "DOWNLOAD_FAILED") }
            }
        }
    }

    @PluginMethod
    fun isModelDownloaded(call: PluginCall) {
        val model = modelFor(call.getString("languageTag") ?: "")
            ?: return call.reject("unknown or unsupported languageTag", "BAD_TAG")
        remoteModelManager.isModelDownloaded(model)
            .addOnSuccessListener { call.resolve(JSObject().put("downloaded", it)) }
            .addOnFailureListener { call.resolve(JSObject().put("downloaded", false)) }
    }

    @PluginMethod
    fun deleteModel(call: PluginCall) {
        val model = modelFor(call.getString("languageTag") ?: "")
            ?: return call.reject("unknown or unsupported languageTag", "BAD_TAG")
        remoteModelManager.deleteDownloadedModel(model)
            .addOnSuccessListener { call.resolve() }
            .addOnFailureListener { e -> call.reject("delete failed: ${e.message}", "DELETE_FAILED") }
    }

    @PluginMethod
    fun recognize(call: PluginCall) {
        val model = modelFor(call.getString("languageTag") ?: "")
            ?: return call.reject("unknown or unsupported languageTag", "BAD_TAG")
        val strokesArr = call.getArray("strokes")
            ?: return call.reject("missing strokes", "BAD_INPUT")
        val area = call.getObject("writingArea")
        val preContext = call.getString("preContext") ?: ""

        val inkBuilder = Ink.builder()
        for (i in 0 until strokesArr.length()) {
            val strokeObj = strokesArr.getJSONObject(i)
            val pts = strokeObj.getJSONArray("points")
            val strokeBuilder = Ink.Stroke.builder()
            for (j in 0 until pts.length()) {
                val p = pts.getJSONObject(j)
                strokeBuilder.addPoint(
                    Ink.Point.create(
                        p.getDouble("x").toFloat(),
                        p.getDouble("y").toFloat(),
                        p.optLong("t", 0L),
                    )
                )
            }
            inkBuilder.addStroke(strokeBuilder.build())
        }
        val ink = inkBuilder.build()

        val recognizer = DigitalInkRecognition.getClient(
            DigitalInkRecognizerOptions.builder(model).build()
        )
        val context = RecognitionContext.builder()
            .setPreContext(preContext)
            .setWritingArea(
                WritingArea(
                    (area?.getInt("width") ?: 0).toFloat(),
                    (area?.getInt("height") ?: 0).toFloat(),
                )
            )
            .build()

        recognizer.recognize(ink, context)
            .addOnSuccessListener { result ->
                val out = JSObject()
                val arr = com.getcapacitor.JSArray()
                for (c in result.candidates) {
                    arr.put(JSObject().put("text", c.text).put("score", c.score ?: 0.0))
                }
                out.put("candidates", arr)
                call.resolve(out)
            }
            .addOnFailureListener { e -> call.reject("recognize failed: ${e.message}", "RECOGNIZE_FAILED") }
    }
}
