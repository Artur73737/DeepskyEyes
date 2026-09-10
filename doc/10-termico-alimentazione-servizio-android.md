# 10 — Termica, alimentazione, servizio Android (ricerca online)

Copre: README §45–48, §97, §99.

## 1. Termica Pixel 8 Pro

- Teardown (PBK Reviews via Notebookcheck/Ars): **niente vapor chamber** (come 7 Pro), ma strati di grafite insolitamente spessi su batteria/RAM/CPU + nastro rame su RAM/CPU + grafite verso chassis. Vapor chamber aiuta la sostenuta ma non compensa inefficienza del SoC (Tensor G3 Samsung 4 nm).
- Long-term review: nessun overheat critico in uso normale con G3, ma camera/4K/gaming scaldano.
- **Adaptive Thermal** (Device Health Services 1.27+, Android 15): warning batteria a **49 °C** (perf ridotte, chiudi app, evita sole), stato emergency a **52 °C** (limita perf, disabilita 5G...), shutdown tra **55 °C + 30 s**. Prima i Pixel si spegnevano senza preavviso.
- Policy DeepskyEyes (§45): monitorare battery temp, thermal status, CPU, camera state, failure; UI `NORMAL/WARM/HOT/THROTTLING/CRITICAL`; policy termica di sequenza (pausa/stop/soglie).
- Checklist §97: idle, +30/60/120 min, dopo 300 frame, throttling/shutdown/instabilità/noise-vs-temp.

## 2. Alimentazione (§46/§99)

USB-C 3.2, 5050 mAh, 27–30 W wired. Da verificare: ricarica durante camera attiva, throttling di carica,
temperatura in carica, screen-off, Doze, background restrictions, foreground requirements, lifetime camera,
negoziazione USB power. Sessioni da ore → test con alimentazione esterna.

## 3. Servizio Android per non morire (§46–48)

- Architettura attesa: `MainActivity → CameraService (CameraManager/Controller/Acquisition/TransportServer)`. Il service possiede l'operazione long-running, non lo screen (schermo ON/UNLOCKED/VISIBLE non richiesti — test esplicito).
- **Foreground service type `camera`** (Android 11+ dichiarazione, Android 14+ permessi `FOREGROUND_SERVICE` + `FOREGROUND_SERVICE_CAMERA` + runtime CAMERA). Non si crea da background senza permessi while-in-use; non da `BOOT_COMPLETED` (ban Android 15 per camera); eccezioni background-start limitate (Android 12+).
- **Doze/App Standby** (API 23+): senza alimentazione + fermo + screen-off → stop rete, ignore wake lock, differita alarm/job/sync. Esenzione solo via `isIgnoringBatteryOptimizations()` / settings o intent `ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS` (Play Policy: solo se core function impattata — il nostro caso). Tenere `PARTIAL_WAKE_LOCK` + foreground + esenzione + alimentato per sequenze 75'+.
- Limiti Android 15: timeout 6 h/24 h per `dataSync/mediaProcessing` (non per camera, ma da conoscere).

## Fonti

- https://www.notebookcheck.net/Pixel-8-Pro-teardown-reveals-no-vapor-chamber-but-thicker-than-usual-graphite-copper-layers-instead.758306.0.html
- https://arstechnica.com/gadgets/2023/10/pixel-8-pro-teardown-reveals-better-cooling-interior-google-branding
- https://www.notebookcheck.net/Google-to-bring-new-quality-of-life-improvements-to-Pixel-8-Pro-and-other-Pixel-devices-with-new-Adaptive-Thermal-warnings.852349.0.html
- https://developer.android.com/develop/background-work/services/fgs/service-types
- https://developer.android.com/develop/background-work/services/fgs/launch
- https://developer.android.com/develop/background-work/services/fgs/restrictions-bg-start
- https://developer.android.com/training/monitoring-device-state/doze-standby
- https://developer.android.com/develop/background-work/services/fgs/changes
- https://www.howtogeek.com/google-pixel-8-pro-review
