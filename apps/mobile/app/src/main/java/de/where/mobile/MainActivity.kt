package de.where.mobile

import android.app.Activity
import android.app.AlertDialog
import android.content.Intent
import android.graphics.Color
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.view.KeyEvent
import android.view.View
import android.view.ViewGroup
import android.view.WindowInsets
import android.view.WindowInsetsController
import android.view.WindowManager
import android.webkit.ValueCallback
import android.webkit.WebChromeClient
import android.webkit.WebView
import android.widget.FrameLayout
import android.widget.Toast

/**
 * Die eigentliche Anwendung: ein Vollbild-Fenster auf die Oberfläche der
 * gewählten Datenbank.
 *
 * Es gibt keine Adresszeile, keine Tableiste und keinen Verlauf nach draußen —
 * die WebView ist die App. `Zurück` fragt, ob es zur Datenbankauswahl
 * zurückgehen soll, statt die eben verlassene Seite wieder aufzublättern.
 */
class MainActivity : Activity() {

    private var web: WebView? = null
    private var dateiZiel: ValueCallback<Array<Uri>>? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val speicher = DatabaseStore(this)
        val gewaehlt = speicher.find(intent.getLongExtra(EXTRA_DATABASE, -1L))
            ?: speicher.all().firstOrNull()
        if (gewaehlt == null) {
            zurAuswahl()
            return
        }
        speicher.select(gewaehlt.id)

        val ansicht = baueWebView(gewaehlt.address)
        web = ansicht
        // Erst der Inhalt: die Vollbild-Regler brauchen das fertige Fenster.
        setContentView(ansicht)
        vollbild()
    }

    override fun onDestroy() {
        web?.let { ansicht ->
            (ansicht.parent as? ViewGroup)?.removeView(ansicht)
            ansicht.destroy()
        }
        web = null
        super.onDestroy()
    }

    override fun onKeyDown(keyCode: Int, event: KeyEvent): Boolean {
        if (keyCode == KeyEvent.KEYCODE_BACK) {
            frageAuswahl()
            return true
        }
        return super.onKeyDown(keyCode, event)
    }

    /** Nach einem Dialog oder der Tastatur steht das Vollbild wieder. */
    override fun onWindowFocusChanged(hasFocus: Boolean) {
        super.onWindowFocusChanged(hasFocus)
        if (hasFocus) vollbild()
    }

    // --------------------------------------------------------------- WebView

    private fun baueWebView(apiBase: String): WebView {
        val ansicht = WebView(this)
        ansicht.layoutParams = FrameLayout.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT,
            ViewGroup.LayoutParams.MATCH_PARENT
        )
        ansicht.setBackgroundColor(Color.WHITE)
        ansicht.settings.apply {
            javaScriptEnabled = true
            domStorageEnabled = true
            allowFileAccess = false
            allowContentAccess = false
            // Ohne `useWideViewPort` ignoriert die WebView das `width=device-width` der
            // Seite; `loadWithOverviewMode` holt eine breitere Desktop-Ansicht auf ein
            // schmales Gerät (Tablet: keine Wirkung, Handy: alles im Blick).
            useWideViewPort = true
            loadWithOverviewMode = true
            // Zwei Finger zum Zoomen ja, sichtbare Zoom-Knöpfe nein.
            setSupportZoom(true)
            builtInZoomControls = true
            displayZoomControls = false
            mediaPlaybackRequiresUserGesture = false
        }
        ansicht.isLongClickable = false
        ansicht.setOnLongClickListener { true }

        ansicht.webViewClient = AppAssets(
            context = this,
            apiBase = apiBase,
            onExternal = { ziel -> meldung("Externer Link nicht verfügbar: $ziel") },
            onError = { text -> meldung("Datenbank nicht erreichbar: $text") }
        )

        ansicht.webChromeClient = object : WebChromeClient() {
            /** CSV-Import und Datei-Uploads der Oberfläche öffnen die Dateiauswahl. */
            override fun onShowFileChooser(
                view: WebView?,
                callback: ValueCallback<Array<Uri>>?,
                params: FileChooserParams?
            ): Boolean {
                dateiZiel?.onReceiveValue(null)
                dateiZiel = callback
                val auswahl = Intent(Intent.ACTION_GET_CONTENT).apply {
                    addCategory(Intent.CATEGORY_OPENABLE)
                    type = "*/*"
                }
                return try {
                    startActivityForResult(
                        Intent.createChooser(auswahl, getString(R.string.datei_waehlen)),
                        REQ_DATEI
                    )
                    true
                } catch (_: Exception) {
                    dateiZiel = null
                    false
                }
            }

            override fun onPermissionRequest(request: android.webkit.PermissionRequest) {
                request.deny()
            }
        }

        ansicht.loadUrl(AppAssets.START)
        return ansicht
    }

    @Deprecated("Die Dateiauswahl der Oberfläche braucht das Ergebnis.")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        if (requestCode == REQ_DATEI) {
            val ziel = dateiZiel
            dateiZiel = null
            ziel?.onReceiveValue(WebChromeClient.FileChooserParams.parseResult(resultCode, data))
            return
        }
        @Suppress("DEPRECATION")
        super.onActivityResult(requestCode, resultCode, data)
    }

    // ------------------------------------------------------------ Kiosk-Rahmen

    @Suppress("DEPRECATION")
    private fun vollbild() {
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            window.setDecorFitsSystemWindows(false)
            window.insetsController?.apply {
                hide(WindowInsets.Type.systemBars())
                systemBarsBehavior = WindowInsetsController.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
            }
        } else {
            window.decorView.systemUiVisibility = (
                View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY
                    or View.SYSTEM_UI_FLAG_FULLSCREEN
                    or View.SYSTEM_UI_FLAG_HIDE_NAVIGATION
                    or View.SYSTEM_UI_FLAG_LAYOUT_STABLE
                    or View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN
                    or View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION
                )
        }
    }

    private fun frageAuswahl() {
        AlertDialog.Builder(this)
            .setTitle(R.string.app_name)
            .setMessage(R.string.zurueck_frage)
            .setPositiveButton(R.string.zur_auswahl) { _, _ -> zurAuswahl() }
            .setNegativeButton(R.string.weiterarbeiten, null)
            .show()
    }

    private fun zurAuswahl() {
        startActivity(Intent(this, SettingsActivity::class.java))
        finish()
    }

    private fun meldung(text: String) {
        Toast.makeText(this, text, Toast.LENGTH_LONG).show()
    }

    companion object {
        /** Welche Datenbank geöffnet werden soll (`Database.id`). */
        const val EXTRA_DATABASE = "database"
        private const val REQ_DATEI = 41
    }
}
