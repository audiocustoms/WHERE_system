package de.where.mobile

import android.content.Context
import android.net.Uri
import android.webkit.WebResourceError
import android.webkit.WebResourceRequest
import android.webkit.WebResourceResponse
import android.webkit.WebView
import android.webkit.WebViewClient
import org.json.JSONObject
import java.io.ByteArrayInputStream
import java.io.InputStream

/**
 * Liefert die gebaute Oberfläche aus `assets/web` unter `https://tauri.localhost`.
 *
 * Der Origin ist derselbe, den die Desktop-Hülle benutzt — die API erlaubt ihn
 * bereits über ihre CORS-Liste, der Server muss also nicht umkonfiguriert
 * werden. Nur `file://` wäre keine Alternative: die Oberfläche lädt ihre Module
 * über absolute Pfade (`/_app/…`).
 *
 * In die `index.html` wird die API-Adresse der gewählten Datenbank eingesetzt,
 * bevor die Oberfläche startet — genau so macht es die Desktop-Hülle.
 */
class AppAssets(
    context: Context,
    private val apiBase: String,
    private val onExternal: (Uri) -> Unit,
    private val onError: (String) -> Unit
) : WebViewClient() {

    private val assets = context.applicationContext.assets

    override fun shouldInterceptRequest(
        view: WebView,
        request: WebResourceRequest
    ): WebResourceResponse? {
        val url = request.url
        if (!istEigeneAdresse(url)) return null

        val methode = request.method ?: "GET"
        if (methode != "GET" && methode != "HEAD") {
            return WebResourceResponse("text/plain", "utf-8", 405, "Method Not Allowed", emptyMap(), null)
        }

        val pfad = pfadAus(url)
        val inhalt = lesen(pfad) ?: lesen("index.html") ?: return null
        val istHtml = pfad == "index.html" || pfad.endsWith("/index.html")
        val bytes = if (istHtml) mitApiAdresse(inhalt) else inhalt
        return WebResourceResponse(
            mimetype(pfad),
            if (istText(pfad)) "utf-8" else null,
            200,
            "OK",
            emptyMap(),
            ByteArrayInputStream(bytes)
        )
    }

    /** Fremde Ziele bleiben draußen: die App ist ein Fenster auf eine Datenbank. */
    override fun shouldOverrideUrlLoading(view: WebView, request: WebResourceRequest): Boolean {
        val url = request.url
        if (istEigeneAdresse(url)) return false
        onExternal(url)
        return true
    }

    /** Meldet einen Fehler nur, wenn die Seite selbst nicht geladen werden konnte. */
    override fun onReceivedError(
        view: WebView,
        request: WebResourceRequest,
        error: WebResourceError
    ) {
        if (!request.isForMainFrame) return
        onError(error.description?.toString().orEmpty())
    }

    private fun istEigeneAdresse(url: Uri): Boolean =
        url.scheme == "https" && url.host == HOST

    /** `/` wird zur Startseite; ein Verzeichnis zu seiner `index.html`. */
    private fun pfadAus(url: Uri): String {
        val roh = (url.path ?: "").trimStart('/')
        return when {
            roh.isEmpty() -> "index.html"
            roh.endsWith("/") -> "${roh}index.html"
            else -> roh
        }
    }

    private fun lesen(pfad: String): ByteArray? = try {
        assets.open("web/${assetName(pfad)}").use(InputStream::readBytes)
    } catch (_: Exception) {
        null
    }

    /**
     * Android lässt Asset-Namen mit führendem `_` weg; der Gradle-Task legt sie
     * deshalb als `x_app` ab. Hier wird der Name wieder zurückgedreht.
     */
    private fun assetName(pfad: String): String =
        pfad.split('/').joinToString("/") { teil -> if (teil.startsWith("_")) "x$teil" else teil }

    private fun mitApiAdresse(html: ByteArray): ByteArray {
        val text = String(html, Charsets.UTF_8)
        val script = "<script>window.__WHERE_API_BASE__=${JSONObject.quote(apiBase)};</script>"
        val stelle = text.indexOf("</head>")
        val neu = if (stelle >= 0) {
            text.substring(0, stelle) + script + text.substring(stelle)
        } else {
            script + text
        }
        return neu.toByteArray(Charsets.UTF_8)
    }

    private fun istText(pfad: String): Boolean = when (erweiterung(pfad)) {
        "html", "js", "mjs", "css", "json", "svg", "txt", "csv", "webmanifest" -> true
        else -> false
    }

    private fun mimetype(pfad: String): String = when (erweiterung(pfad)) {
        "html" -> "text/html"
        "js", "mjs" -> "text/javascript"
        "css" -> "text/css"
        "json", "webmanifest" -> "application/json"
        "svg" -> "image/svg+xml"
        "png" -> "image/png"
        "jpg", "jpeg" -> "image/jpeg"
        "gif" -> "image/gif"
        "webp" -> "image/webp"
        "ico" -> "image/x-icon"
        "woff" -> "font/woff"
        "woff2" -> "font/woff2"
        "ttf" -> "font/ttf"
        "txt" -> "text/plain"
        "csv" -> "text/csv"
        "wasm" -> "application/wasm"
        else -> "application/octet-stream"
    }

    private fun erweiterung(pfad: String): String =
        pfad.substringAfterLast('.', "").lowercase()

    companion object {
        const val HOST = "tauri.localhost"
        const val START = "https://$HOST/"
    }
}
