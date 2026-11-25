# WHERE mobile (Android)

The Android app is a **window on one database**: it shows exactly the same interface as
`http://localhost:5173/`, but without browser chrome — no address bar, no tabs, no way out
except the database list.

```
Start → Datenbanken  (list of servers stored on this device)
      → tap an entry → full-screen kiosk window with the WHERE interface
```

## What the app does

- **Datenbanken** (start screen, [SettingsActivity.kt](app/src/main/java/de/where/mobile/SettingsActivity.kt)):
  add, edit and remove the addresses of WHERE servers. The list lives in the app's
  SharedPreferences ([DatabaseStore.kt](app/src/main/java/de/where/mobile/DatabaseStore.kt)), so
  one tablet can point at several installations — e.g. one per tenant.
  `buero-server` becomes `http://buero-server:8090`, a missing port is filled in with `8090`.
- **Kiosk window** ([MainActivity.kt](app/src/main/java/de/where/mobile/MainActivity.kt)):
  full screen, screen stays awake, `Zurück` does not page back but asks whether to return to
  the database list. Rotation does not reload the page. The WebView honours the page's
  `width=device-width` and zooms a wide desktop layout down to a narrow phone screen
  (`useWideViewPort` + `loadWithOverviewMode`).
- **File uploads** (CSV import, attachments) open the Android file picker.
- **External links** are refused — a tap on a foreign link only shows a hint.

## How the interface gets there

The app carries the built SvelteKit SPA in its assets and serves it to the WebView under
`https://tauri.localhost` ([AppAssets.kt](app/src/main/java/de/where/mobile/AppAssets.kt)).
That is the same origin the Tauri desktop shell uses, and the API already allows it in its
CORS list (`http://tauri.localhost`, `https://tauri.localhost`, `tauri://localhost`) — the
server needs no extra configuration.

`file://` would not work: the SPA loads its modules from absolute paths (`/_app/…`). Before
the SPA starts, the chosen server address is written into the page as
`window.__WHERE_API_BASE__` — the same hook the desktop shell uses, so
[api.ts](../web/src/lib/api.ts) needs no change.

One detail worth knowing: Android leaves asset names with a leading `_` out of the APK, and
SvelteKit puts everything under `_app`. The Gradle task therefore stores those segments as
`x_app` and `AppAssets` maps the name back when serving — `/_app/…` becomes
`assets/web/x_app/…` inside the APK.

Two consequences of that design:

- The server must be reachable over **plain HTTP** (LAN); the manifest sets
  `usesCleartextTraffic="true"` for that. A server behind HTTPS works as well.
- The interface inside the app is the build that was packaged. A new frontend means a new
  APK, just like the desktop shell.

## Building

Prerequisites: JDK 17+ (tested with 21), Android SDK with platform 36 and build-tools 36,
and the web build.

```powershell
# 1. Build the shared interface
cd apps/web
npm run build

# 2. Build the app (local.properties gets sdk.dir, or set ANDROID_HOME)
cd ../mobile
$env:JAVA_HOME = "C:\Program Files\Android\openjdk\jdk-21.0.8"
.\gradlew.bat :app:assembleDebug
# → app/build/outputs/apk/debug/app-debug.apk
```

`local.properties` is not versioned — Android Studio writes it, or set `ANDROID_HOME`.
`syncWeb` copies `apps/web/build` into `app/src/main/assets/web` (not versioned either); if
the web build is missing the Gradle build stops with that hint. Unit tests cover the address
normalisation: `.\gradlew.bat :app:testDebugUnitTest`.

Android Studio: open `apps/mobile` as a project and run the `app` configuration.

The launcher icon of the app comes from `logo/logo.svg` like every other icon of the family
(see the main [README](../../README.md#icons-and-logos)); `res/drawable/ic_launcher.xml` still
carries the card box that was drawn for the first build and is replaced by the logo mark with
the next build.

## Layout

| Path | Content |
|---|---|
| `app/src/main/java/de/where/mobile/SettingsActivity.kt` | start screen: database list |
| `app/src/main/java/de/where/mobile/MainActivity.kt` | kiosk window, file chooser, back handling |
| `app/src/main/java/de/where/mobile/AppAssets.kt` | serves `assets/web` under `https://tauri.localhost` |
| `app/src/main/java/de/where/mobile/DatabaseStore.kt` | list of databases, address normalisation |
| `app/src/main/res/layout/` | `activity_settings`, `item_database`, `dialog_database` |

## Checked on an emulator

Run on Android 12 (MuMuPlayer, 1080 × 1920, 480 dpi) against the local API: adding a database
(`127.0.0.1:8090` became `http://127.0.0.1:8090`), the kiosk window loading the bundled
interface, the tenant and user list coming from the API, the login and the `Zentrale` with
live counts, the full-screen mode, the `Zurück` question and the return to the list with the
`zuletzt verwendet` mark. The API was reachable from the emulator through
`adb reverse tcp:8090 tcp:8090`.
