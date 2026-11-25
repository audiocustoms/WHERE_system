package de.where.mobile

import android.app.Activity
import android.app.AlertDialog
import android.content.DialogInterface
import android.content.Intent
import android.os.Bundle
import android.view.View
import android.view.ViewGroup
import android.widget.BaseAdapter
import android.widget.Button
import android.widget.EditText
import android.widget.ListView
import android.widget.TextView

/**
 * Die Startseite der App: die hinterlegten Datenbanken und die Auswahl.
 *
 * Ein Eintrag wird angetippt und öffnet die Oberfläche; das Stift-Symbol
 * bearbeitet Name und Adresse, im Bearbeiten-Dialog lässt sich der Eintrag
 * wieder entfernen.
 */
class SettingsActivity : Activity() {

    private lateinit var speicher: DatabaseStore
    private lateinit var liste: ListView
    private lateinit var hinweis: TextView
    private var eintraege: List<Database> = emptyList()

    private val adapter = object : BaseAdapter() {
        override fun getCount(): Int = eintraege.size

        override fun getItem(position: Int): Any = eintraege[position]

        override fun getItemId(position: Int): Long = eintraege[position].id

        override fun getView(position: Int, convertView: View?, parent: ViewGroup?): View {
            val zeile = convertView
                ?: layoutInflater.inflate(R.layout.item_database, parent, false)
            val datenbank = eintraege[position]
            val zuletzt = datenbank.id == speicher.selectedId()

            zeile.findViewById<TextView>(R.id.item_name).text = datenbank.name
            zeile.findViewById<TextView>(R.id.item_address).text =
                if (zuletzt) {
                    getString(R.string.zeile_adresse_zuletzt, datenbank.address)
                } else {
                    datenbank.address
                }
            zeile.findViewById<Button>(R.id.item_edit).setOnClickListener { bearbeiten(datenbank) }
            return zeile
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_settings)
        speicher = DatabaseStore(this)

        liste = findViewById(R.id.datenbanken)
        hinweis = findViewById(R.id.hinweis)
        liste.adapter = adapter
        liste.setOnItemClickListener { _, _, position, _ -> oeffne(eintraege[position]) }

        findViewById<Button>(R.id.hinzufuegen).setOnClickListener { bearbeiten(null) }
        laden()
    }

    private fun laden() {
        eintraege = speicher.all()
        adapter.notifyDataSetChanged()
        hinweis.visibility = if (eintraege.isEmpty()) View.VISIBLE else View.GONE
    }

    private fun oeffne(datenbank: Database) {
        speicher.select(datenbank.id)
        startActivity(
            Intent(this, MainActivity::class.java)
                .putExtra(MainActivity.EXTRA_DATABASE, datenbank.id)
        )
    }

    private fun bearbeiten(vorlage: Database?) {
        val formular = layoutInflater.inflate(R.layout.dialog_database, null)
        val nameFeld = formular.findViewById<EditText>(R.id.dialog_name)
        val adresseFeld = formular.findViewById<EditText>(R.id.dialog_address)
        if (vorlage != null) {
            nameFeld.setText(vorlage.name)
            adresseFeld.setText(vorlage.address)
        }

        val dialog = AlertDialog.Builder(this)
            .setTitle(if (vorlage == null) R.string.neu else R.string.bearbeiten)
            .setView(formular)
            .setPositiveButton(R.string.sichern, null)
            .setNegativeButton(R.string.abbrechen, null)
            .create()

        if (vorlage != null) {
            dialog.setButton(DialogInterface.BUTTON_NEUTRAL, getString(R.string.loeschen)) { _, _ ->
                entfernen(vorlage)
            }
        }

        // Erst prüfen, dann schließen: ein leeres Feld soll den Dialog offen lassen.
        dialog.setOnShowListener {
            dialog.getButton(DialogInterface.BUTTON_POSITIVE).setOnClickListener {
                val adresse = DatabaseStore.normalize(adresseFeld.text.toString())
                if (adresse.isEmpty()) {
                    adresseFeld.error = getString(R.string.adresse_fehlt)
                    return@setOnClickListener
                }
                speicher.save(
                    Database(
                        id = vorlage?.id ?: System.currentTimeMillis(),
                        name = nameFeld.text.toString().trim().ifEmpty { adresse },
                        address = adresse
                    )
                )
                laden()
                dialog.dismiss()
            }
        }
        dialog.show()
    }

    private fun entfernen(datenbank: Database) {
        AlertDialog.Builder(this)
            .setMessage(getString(R.string.loeschen_frage, datenbank.name))
            .setPositiveButton(R.string.loeschen) { _, _ ->
                speicher.remove(datenbank.id)
                laden()
            }
            .setNegativeButton(R.string.abbrechen, null)
            .show()
    }
}
