package de.where.mobile

import android.content.Context
import org.json.JSONArray
import org.json.JSONObject

/**
 * Ein Eintrag der Datenbankliste: der Name, unter dem der Anwender die Datenbank
 * erkennt, und die Adresse ihres WHERE-Servers.
 */
data class Database(val id: Long, val name: String, val address: String)

/**
 * Hält die Datenbanken und die zuletzt gewählte in den SharedPreferences.
 *
 * Die Liste ist klein und gehört nur dieser App — eine Datei oder gar eine
 * eigene Datenbank wäre dafür zu viel.
 */
class DatabaseStore(context: Context) {

    private val prefs =
        context.applicationContext.getSharedPreferences("where.databases", Context.MODE_PRIVATE)

    fun all(): List<Database> {
        val roh = prefs.getString(KEY_LIST, null) ?: return emptyList()
        return try {
            val array = JSONArray(roh)
            (0 until array.length()).mapNotNull { index ->
                val eintrag = array.optJSONObject(index) ?: return@mapNotNull null
                val adresse = eintrag.optString("address").trim()
                if (adresse.isEmpty()) {
                    null
                } else {
                    Database(
                        id = eintrag.optLong("id", index.toLong()),
                        name = eintrag.optString("name").trim().ifEmpty { adresse },
                        address = adresse
                    )
                }
            }
        } catch (_: Exception) {
            emptyList()
        }
    }

    fun find(id: Long): Database? = all().firstOrNull { it.id == id }

    /** Legt einen Eintrag an oder ersetzt ihn; die Reihenfolge bleibt erhalten. */
    fun save(database: Database) {
        val liste = all().toMutableList()
        val stelle = liste.indexOfFirst { it.id == database.id }
        if (stelle >= 0) liste[stelle] = database else liste.add(database)
        write(liste)
    }

    fun remove(id: Long) {
        write(all().filterNot { it.id == id })
        if (selectedId() == id) prefs.edit().remove(KEY_SELECTED).apply()
    }

    /** Die Auswahl überlebt einen Neustart, damit ein Kiosk-Gerät wieder dort landet. */
    fun selectedId(): Long = prefs.getLong(KEY_SELECTED, -1L)

    fun select(id: Long) {
        prefs.edit().putLong(KEY_SELECTED, id).apply()
    }

    private fun write(liste: List<Database>) {
        val array = JSONArray()
        for (eintrag in liste) {
            array.put(
                JSONObject()
                    .put("id", eintrag.id)
                    .put("name", eintrag.name)
                    .put("address", eintrag.address)
            )
        }
        prefs.edit().putString(KEY_LIST, array.toString()).apply()
    }

    companion object {
        private const val KEY_LIST = "list"
        private const val KEY_SELECTED = "selected"

        /**
         * Macht aus einer getippten Adresse eine brauchbare Server-Adresse:
         * fehlendes Schema wird `http://`, bei `http` wird ein fehlender Port zu
         * `8090`, ein abschließender Schrägstrich fällt weg. `https` bleibt ohne
         * Port — dort gilt 443.
         */
        fun normalize(raw: String): String {
            var text = raw.trim()
            if (text.isEmpty()) return ""
            if (!text.contains("://")) text = "http://$text"

            val schemaEnde = text.indexOf("://") + 3
            val schema = text.substring(0, schemaEnde - 3).lowercase()
            val pfadStart = text.indexOf('/', schemaEnde).let { if (it < 0) text.length else it }

            val autoritaet = text.substring(schemaEnde, pfadStart)
            if (schema == "http" && autoritaet.isNotEmpty() && !autoritaet.contains(':')) {
                text = text.substring(0, pfadStart) + ":8090" + text.substring(pfadStart)
            }
            return text.trimEnd('/')
        }
    }
}
