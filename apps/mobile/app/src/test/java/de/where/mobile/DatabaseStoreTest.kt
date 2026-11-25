package de.where.mobile

import org.junit.Assert.assertEquals
import org.junit.Test

/** Die Adressen kommen von Hand getippt — hier wird die Eingabe geglättet. */
class DatabaseStoreTest {

    @Test
    fun `hostname bekommt Schema und Port`() {
        assertEquals("http://buero-server:8090", DatabaseStore.normalize("buero-server"))
    }

    @Test
    fun `eingetragener Port bleibt`() {
        assertEquals("http://192.168.1.10:8080", DatabaseStore.normalize("http://192.168.1.10:8080"))
    }

    @Test
    fun `abschliessender Schraegstrich faellt weg`() {
        assertEquals("http://server:8090", DatabaseStore.normalize("  http://server:8090/  "))
    }

    @Test
    fun `https bleibt ohne Port`() {
        assertEquals("https://where.example.org", DatabaseStore.normalize("https://where.example.org"))
    }

    @Test
    fun `pfad bleibt erhalten`() {
        assertEquals("http://server:8090/where", DatabaseStore.normalize("server/where"))
    }

    @Test
    fun `leere Eingabe bleibt leer`() {
        assertEquals("", DatabaseStore.normalize("   "))
    }
}
