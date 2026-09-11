package com.deepskyeyes.android.protocol

import org.json.JSONArray
import org.json.JSONObject

fun obj(vararg pairs: Pair<String, Any?>): JSONObject = JSONObject().apply {
    pairs.forEach { (key, value) -> put(key, value ?: JSONObject.NULL) }
}
fun arr(items: Iterable<*>): JSONArray = JSONArray().apply { items.forEach { put(it) } }
fun JSONObject.objectOrNull(key: String): JSONObject? = optJSONObject(key)
fun JSONObject.longOrNull(key: String): Long? = if (isNull(key)) null else getLong(key)
fun JSONObject.stringOrNull(key: String): String? = if (isNull(key)) null else getString(key)
fun JSONArray.objects(): List<JSONObject> = (0 until length()).map { getJSONObject(it) }
fun JSONArray.strings(): List<String> = (0 until length()).map { getString(it) }
fun JSONObject.copyJson(): JSONObject = JSONObject(toString())
fun JSONObject.sameJson(other: JSONObject): Boolean {
    val keys = keys().asSequence().toSet()
    if (keys != other.keys().asSequence().toSet()) return false
    return keys.all { key -> equalJson(get(key),other.get(key)) }
}
private fun equalJson(a: Any?,b: Any?): Boolean = when {
    a is JSONObject && b is JSONObject -> a.sameJson(b)
    a is JSONArray && b is JSONArray -> a.length() == b.length() && (0 until a.length()).all { equalJson(a.get(it),b.get(it)) }
    a is Number && b is Number -> a.toString() == b.toString()
    else -> a == b
}
class CameraFault(val code: String, message: String) : Exception(message)
fun fault(code: String, message: String): Nothing = throw CameraFault(code, message)
fun requireCamera(condition: Boolean, code: String, message: String) { if (!condition) fault(code, message) }
