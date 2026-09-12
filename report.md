# DeepskyEyes — Report finale controllo totale Pixel 8 Pro (ADB, exe Rust)

Data: 2026-09-12 · Telefono `44101FDJG003S3` (Pixel 8 Pro / Android 17) · Exe `Rust_App/target/release/deepsky-eyes.exe` · App Kotlin avviata.
Piano completo: `test.md` (T0–T15). Qui solo l'esito dettagliato voce per voce.

## Verdetto in 30 secondi
**Controllo totale: SÌ per esposizione, ISO, fuoco manuale, WB preset, RAW/DNG, preview, sessioni. NO (by-design) per ISO-auto, exp-auto, WB-auto, Kelvin: rifiutati con errore esplicito. SORPRESA: AF one-shot funziona via `preview --autofocus` anche se `caps.af_modes=[off]`. GAP CLI: mancano `--camera` su capture/sequence, `--zoom`, `--stream`, `--ois/eis/processing`, `--frame-duration`.** Nessun clamp silenzioso osservato in 20+ scatti: ogni fuori-range dà `OutOfRange/Unsupported/ValidationFailed`.

## 1. Connessione e discovery
| Cosa | Esito | Dettaglio |
|---|---|---|
| ADB + forward | PASS | `44101FDJG003S3 device`, `discover` → `0 back + 1 front`, origin=Device |
| `camera list` | PASS | `0 … streams=23 physical=[2,3,4,5,6]`, `1 … streams=12 physical=[7,8]` |
| `capabilities 0` | PASS | exp 26345..16000001084, iso 21..10666, focus 0..9523, zoom 495..30000, Dng+Raw16Le 4080x3072/2288/2032x1536/2016x1136, wb 8 modi, ois/eis off+on |
| `capabilities 1` | PASS | exp 46377..1000003181 (~1s), iso 51..4873, Dng+Raw16Le 3440x2448 |
| camera `99` | PASS (fail atteso) | `error: camera '99' not announced` EXIT:1 |

## 2. Esposizione — CONTROLLO TOTALE ✓
| Test | Comando | Esito |
|---|---|---|
| 1s (riferimento) | `sequence --frames 10 --exposure-ns 1000000000` | PASS: 10/10 DNG 24MB, req 1000000000 → rep 999971248 (10x1s) / 99999644 (100ms) |
| 100ms | `capture --exposure-ns 100000000` | PASS, preview luma 38, stanza visibile |
| min 26345ns | `capture --exposure-ns 26345` | PASS, 1 DNG, luma 0.9 (nero quasi totale, corretto) |
| oltre max 30s | `--exposure-ns 30000000000` | PASS (fail atteso): `camera: Io: OutOfRange: exposure_ns` |
| 0 | `--frames 0` analogo | `ValidationFailed(empty plan)` / `frames must be positive` |
| exp-auto | — | GAP documentato: nessun flag auto, `exposure_ns` sempre `u64` obbligatorio. Corretto per astro. |

Stabilità: 3 scatti identici 100ms → sempre rep 99999644. Nessun clamp silenzioso.

## 3. ISO / sensibilità — CONTROLLO TOTALE ✓ (solo manuale)
| Test | Esito |
|---|---|
| 800 → 799 | PASS (tolleranza HAL ±2, già nota) |
| min 21 → 21 | PASS, 1 DNG luma 4.7 |
| max 10666 → 10664 | PASS, 1 DNG luma 247 (bruciato a 100ms, corretto) |
| 20000 | PASS (fail atteso): `Io: OutOfRange: sensitivity` |
| 10 (sotto min) | non eseguito, stesso codice di 20000 |
| ISO-auto | GAP by-design: nessun modo auto annunciato né flag. Giusto così per deep-sky. |

## 4. Fuoco + autofocus — CONTROLLO TOTALE con SORPRESA ⚠️→✓
| Test | Esito |
|---|---|
| manual 0 (infinito) locked | PASS, `focus_mode=manual`, 0.0→0.0 |
| medio 4761 | PASS, 4.761→4.775 (+14 millidiottrie di tolleranza HAL, accettabile) |
| max 9523 | PASS, 1 DNG |
| 20000 | PASS (fail atteso): `OutOfRange: focus_millidiopters` |
| `preview --autofocus` | **SORPRESA PASS: `Center AF: 508 millidiopters`, preview 640x480 luma 63.5.** L'HAL supporta `AF_MODE_AUTO` (l'engine lo interroga direttamente), anche se `CapabilityDiscovery.kt:74` filtra `caps.af_modes` a solo `["off"]`. |
| riuso 508 come manuale | PASS: `capture --focus-mdiopt 508` → 0.508→0.508 perfetto |
| sequenza senza lock | non eseguita via CLI (lock sempre true), validazione presente nel codice |

**Raccomandazione:** workflow astro corretto = `preview --autofocus` una volta → riusa il valore in `capture/sequence --focus-mdiopt N`. Valutare se esporre `af_modes=[off,auto]` nelle caps o tenere nascosto di proposito.

## 5. Bilanciamento bianco + Kelvin — CONTROLLO TOTALE preset, AUTO rifiutato by-design ✓
| Test | Esito |
|---|---|
| `daylight` | PASS (10x1s + N custom), `wb=daylight` riportato |
| `shade` | PASS, `wb=shade` |
| `twilight` | PASS, 1 DNG |
| `alien` | PASS (fail atteso): `Unsupported: white-balance preset not announced` |
| `auto` | PASS (fail atteso, 2 volte): `sequencer: ValidationFailed(fixed white balance required)` — il sequencer accetta solo preset fissi, mai `auto` |
| `wb-kelvin 5000` | GAP by-device: `wb_kelvin=null` sempre, nessun modo `temperature`. Il flag esiste ma viene ignorato (mapped su preset). Da documentare in `--help`. |
| restanti `cloudy_daylight, incandescent, fluorescent, warm_fluorescent` | non eseguiti singolarmente, stesso ramo di codice di shade/twilight (rischio basso) |

## 6. Zoom / crop — RISOLTO per zoom, GAP per crop ✓/✗
`--zoom RATIO` (es. `2.0`) e `--zoom-x1000 N` implementati in `controller::parse_zoom`, validati con Reject: `--zoom 2.0` → req 2.0 rep 2.0 ✓; `--zoom 30.1` → `OutOfRange: zoom_x1000` ✓; entrambi insieme → `exclusive` ✓. Resta GAP `--crop` (riportato `[0,0,4080,3072]`, non impostabile).

## 7. Stream / risoluzione — RISOLTO per selezione, BUG-001 su DNG binnato ✓/!
`--stream WxH:FORMAT[:MODE]` implementato (`controller::resolve_stream`, match esatto o errore con lista annunciata). Verificato: `4080x3072:Raw16Le` → `.raw16` full ✓. **BUG-001**: `2032x1536:Dng` → `AssertionError` in `DngCreator.nativeWriteImage` → crash processo 14:14:22 (il catch su `Exception` non prende `AssertionError`). Fix: errore esplicito `Unsupported(DNG writer rejected…; use Raw16Le…)` + catch `Throwable` nel bridge. APK reinstallata, da riverificare. JPEG resta RAW-only per capture (errore esplicito).

## 8. RAW / DNG — OK ✓
DNG Android (`DngCreator`) 4080x3072, ~25MB, sha256 in manifest + `.dng.json` con `requested vs reported`, `physical_camera_id=2`, `crop`, `zoom 1.0`, `ois off`, `processing`, `timestamp_domain=elapsedRealtimeNanos`. Si aprono in Siril/LR (Bayer + matrici). `Raw16Le` diretto non testabile (vedi §7).

## 9. Preview — CONTROLLO TOTALE ✓
`preview --camera 0/1` ok (640x480 Rgb8, `reported_exposure/ISO/format` stampati, `sensor_delta`). Cam 1 front ok (luma 19.2). `--frames` multipli e cam 99 non ancora eseguiti. Nota: a 1s ISO800 indoor la preview è bruciata (luma 244), a 100ms ISO100 è corretta (luma 38, stanza visibile) — comportamento atteso.

## 10. Processing / OIS / EIS — default OK, controllo mancante ✗
Riportati e sani: `aberration off, distortion off, edge off, hot_pixel high_quality, noise_reduction off, shading high_quality, tonemap fast, ois off`. Ma CLI non espone flag: usa `off` dove disponibile e i default HAL altrove. `hot_pixel/shading high_quality` e `tonemap fast` (senza `off`) sono imposti dal device, non scegliibili. Da implementare `--ois/--eis/--processing` per test mosso astronomico.

## 11. Sequencer / sessioni — OK ✓
`sequence 10x1s` → 10/10, `session.json` con `capability_snapshot, thermal_events, warnings (origin Synthetic solo su sim), errors`, `preview.png` finale, sha256/frame, cartelle `lights/darks/flats/bias/test` + filename deterministico `PROJ_STAMP_L_NNNN.dng` (verificati `light/dark/test`). `--delay-ns 0` usato ovunque; confronto delay 0 vs 1s non eseguito. Recovery kill/resume non eseguito (prossimo).

## 12. Errori — TUTTI ESPLICITI ✓
| Input sbagliato | Risposta | Giudizio |
|---|---|---|
| WB auto | `ValidationFailed(fixed white balance required)` | corretto |
| WB alien | `Unsupported: white-balance preset not announced` | corretto |
| exp 30s | `Io: OutOfRange: exposure_ns` | corretto |
| iso 20000 | `Io: OutOfRange: sensitivity` | corretto |
| focus 20000 | `Io: OutOfRange: focus_millidiopters` | corretto |
| frames 0 | `config: frames must be positive` | corretto |
| camera 99 | `camera '99' not announced` | corretto |

## 13. File prodotti (affianco exe)
- `foto_telefono_1s/PIXEL_1S_2026-09-12T120221Z_852809200/lights/*.dng` (10×24MB) + `.dng.json` + `preview.png` + `session.json`
- `foto_1s/.../lights/*.raw16` (10 sim) + `foto_1s_colore/*_colore.png` (debayer demo)
- `foto_test/` 10+ sessioni custom (ISOMIN/ISOMAX/FOCUSMID/FOCUSAF508/FOCUSMAX/WBSHADE/WBTWI/EXPMIN/DARK/TEXP) + `af.png` + `front.png` + `foto_test_preview_100ms.png`

## 14. Cosa manca per “ogni parametro funziona”
1. ~~`--camera` su `capture/sequence`~~ FATTO (verificato cam 1 front).
2. ~~`--zoom`, `--stream`~~ FATTI e verificati. Restano `--crop`, `--ois/--eis/--processing`, `--frame-duration-ns`.
3. `thermal/status/capability_dump` da CLI.
4. Documentare `--wb-kelvin` come no-op su Pixel (o rimuoverlo dall'help).
5. Decidere se pubblicizzare AF in caps (`[off,auto-one-shot]`) visto che funziona.
6. Riverifiche dopo restart service (BUG-001 fix): `2032x1536:Dng` → ora deve dare errore esplicito; `2032x1536:Raw16Le`; `9999x9999:Dng` → `Unsupported` con lista; `preview --frames 5`; WB restanti; 16s max; termico 30x1s; screen-off/Doze.

---

## Appendice 2026-09-12 ~14:05–14:17 — implementazione --camera/--zoom/--stream + BUG-001

### Cosa è stato implementato (su richiesta "procedi")
Nuovi flag CLI in `deepsky-eyes capture/sequence/preview` e `deepsky-app --headless`:
```
[--camera ID] [--zoom RATIO | --zoom-x1000 N] [--stream WxH:FORMAT[:MODE]]
```
- `Rust_App/crates/deepsky-app/src/controller.rs`: nuove funzioni condivise `select_camera()` (ID annunciato o errore, mai sostituzione silenziosa), `parse_zoom()` (`--zoom 2.0` ratio vs `--zoom-x1000 2000` esatto, entrambi insieme = errore), `resolve_stream()` (match esatto width/height/format/mode contro annunciati, errore con lista annunciata). `AcquisitionOptions` ha ora `camera_id: Option<String>`; `run_acquisition()` seleziona la camera richiesta.
- `Rust_App/crates/deepsky-cli/src/commands/acquire.rs`: riscritto per pre-discovery (serve per risolvere `--stream` contro le caps) poi `run_acquisition()` riconnette e rivalida tutto con Reject.
- `preview.rs`: stessi flag (+ `--camera` ora usa `select_camera`, stesso messaggio `not announced`).
- `main.rs` (cli + app headless) e `worker.rs`: help e campi aggiornati.
- Build release ok, `cargo test -p deepsky-app -p deepsky-cli -p deepsky-camera`: 14+3 pass, 1 ignored (hardware).

### Verifiche ADB reali (Pixel 8 Pro 44101FDJG003S3)
| Test | Esito |
|---|---|
| `--camera 1` capture 100ms ISO100 daylight | PASS: DNG front 3440x2448, exp 100000000→99993645 (`foto_test/TCAM1_*`) |
| `--zoom 2.0` | PASS: zoom req 2.0 → rep 2.0, crop full (`foto_test/TZOOM2_*`) |
| `--zoom 30.1` (oltre max 30.0) | PASS fail atteso: `OutOfRange: zoom_x1000` |
| `--zoom 2.0 --zoom-x1000 2000` | PASS fail atteso: `--zoom and --zoom-x1000 are exclusive` |
| `--stream 4080x3072:Raw16Le` | PASS: `.raw16` full + sidecar (`foto_test/TRAW16_*`) |
| `--stream 2032x1536:Dng` | **BUG-001** (vedi sotto) |
| `--stream 9999x9999:Dng` | non conclusivo (server già morto per BUG-001 → `NotConnected`), da ripetere |

### BUG-001 — DNG binnato uccide l'app (trovato dai nuovi flag)
- Sintomo: `--stream 2032x1536:Dng` → CLI `sequencer: RetryExhausted`, poi OGNI comando (persino `discover`) → `Io: NotConnected`.
- Causa (logcat 14:14:22): `FATAL EXCEPTION Deepsky-bridge`, `AssertionError` da `DngCreator.nativeWriteImage` ("image buffer did not match preCorrectionActiveArraySize or pixelArraySize"). Il `DngCreator` accetta solo il full-array; il `catch (e: Exception)` di `BridgeServer` non intercetta gli `Error`, quindi il thread muore e l'intero processo `com.deepskyeyes.android` viene killato (pid sparito, porta 7878 chiusa).
- Fix applicato: `CameraEngine.capture()` converte il rifiuto DNG in `fault("Unsupported", "DNG writer rejected WxH; use Raw16Le for this size")`; `BridgeServer.handle()` cattura `Throwable` così una richiesta non uccide mai il bridge (errore esplicito al desktop, mai socket morto).
- APK rebuildata (`./gradlew assembleDebug`, BUILD SUCCESSFUL) e reinstallata (`adb install -r … Success`, 27MB).
- MainActivity rilanciata via adb; **resta da premere "Start service" sul telefono** e riverificare: `2032x1536:Dng` (ora errore pulito), `2032x1536:Raw16Le`, `9999x9999:Dng` (`Unsupported` + lista), `preview --frames 5`.

### File prodotti in questa sessione
- `foto_test/TCAM1_*`, `TZOOM2_*`, `TRAW16_*` (raw16 full), `TDNGBIN_*` (sessione vuota, prova del crash), `af.png`, `front.png` in `Rust_App/target/release/foto_test/`.
- Sorgenti: `controller.rs` (+~110 righe), `acquire.rs` (riscritto), `preview.rs`, `main.rs` ×2, `worker.rs`, `CameraEngine.kt`, `BridgeServer.kt`.
- Stato finale: in attesa del tap "Start service" per chiudere le riverifiche T7/T9.

## Continuazione CLI + Kotlin — 2026-09-12, dopo il passaggio dell'agente esterno

Lavoro eseguito solo da shell/ADB, senza agenti delegati e senza interazione visuale col PC.
Le sezioni precedenti restano come cronologia: alcune conclusioni erano provvisorie.

### Implementato

- Sorgente CLI predefinita ADB reale; simulatore solo con `--source sim` esplicito.
- Rifiuto di flag sconosciuti, duplicati e senza valore nei comandi camera principali.
- `--frame-duration-ns`, `--crop x,y,width,height`, `--ois`, `--eis`,
  `--processing key=mode,key=mode` per capture/sequence/preview.
- Kelvin esplicito produce ora Unsupported sul Pixel, mai un preset sostitutivo silenzioso.
- `status`, `ping`, `thermal`, `capability_dump`, `autofocus` con output JSON.
- `request-template` genera un CaptureRequest da discovery; `execute --request FILE --out FILE`
  esegue il contratto completo (incluse richieste JPEG) e salva payload e metadati originali.
  I controlli non implementati/non annunciati sono rifiutati; non esiste un bypass dei limiti Camera2.
- `session-inspect --path DIR` legge l'ultima revisione immutabile e verifica hash/integrità.
- Sidecar scientifici: `extra.requested_settings`, `applied_settings`, `reported_settings`, inclusa EIS.
- `--strict-results`: conserva il frame come evidenza e ferma la sequenza se i controlli osservati
  divergono; modalità normale stampa/salva gli avvisi. Quantizzazione: 1%, almeno 2 unità ISO;
  fuoco 50 millidiottrie. Crop e modi discreti confrontati esattamente.
- Ritardo sequenza predefinito zero; niente ritardo aggiuntivo dopo l'ultimo frame.
- Campioni termici periodici ora persistiti nel manifest, non solo letti e scartati.
- Avvio servizio Kotlin dal PC:
  `adb -s 44101FDJG003S3 shell am start -n com.deepskyeyes.android/.MainActivity -a com.deepskyeyes.android.START_BRIDGE`.
  Restano necessari autorizzazione USB, permesso camera e condizioni foreground imposte da Android.
- Discovery DNG filtrata per dimensioni compatibili con DngCreator; RAW16 binnato resta annunciato.
- Autofocus centrale dichiarato separatamente dai modi AF di acquisizione manuale.
- Corretto bug del dump: `android.util.Rational` produceva JSON invalido (`1/6` senza virgolette).
  Adesso conserva `{numerator, denominator}`; float non finiti non invalidano più il documento.
- Errori RPC mantengono il codice camera originale anziché diventare tutti Io/retry.

### Prima matrice reale completata

Evidenze: `Rust_App/target/release/captures/CLI_AUDIT_20260912_143032_129/results.json`.
24 invocazioni con exit code atteso: stato, ping, termico, dump, AF, errori intenzionali,
RAW16 2032x1536, controlli avanzati, crop, quattro WB restanti, cinque preview,
template JPEG, JPEG reale e rifiuto overwrite. Script ripetibile: `doc/cli-regression.ps1`.

**Non confondere richiesta accettata con controllo rispettato dall'HAL:**

| Parametro | Richiesto | Risultato reale |
| --- | --- | --- |
| Durata frame | 200000000 ns | 200003149 ns |
| Esposizione | 100000000 ns | 99999644 ns |
| OIS / EIS | on / on | on / on |
| edge, noise_reduction, distortion | fast | fast |
| tonemap | high_quality | high_quality |
| hot_pixel, shading | fast | **high_quality**, HAL non rispetta questa richiesta su RAW |
| Crop | 1020,768,2040,1536 | 1021,769,2039,1535 (arrotondamento HAL) |

Il DNG conserva 4080x3072 pixel; il crop Camera2 non equivale a un ritaglio software
del buffer RAW. WB incandescent/fluorescent/warm_fluorescent/cloudy_daylight acquisiti.
Temperatura numerica non disponibile (`null`), severità nominale: non inventare gradi Celsius.

### Limiti dichiarati

Questo è controllo del contratto Camera2 implementato, non accesso ai controlli privati
della Google Camera. AE/ISO automatici, routing fisico, manual gains/Kelvin non sono
annunciati/implementati dall'adapter corrente; `execute` non li rende magicamente disponibili.
Le sequenze scientifiche mantengono RAW, fuoco bloccato e WB fisso; JPEG e AWB auto
possono essere diagnosticati tramite `execute` senza indebolire il preflight scientifico.
Il dump esteso distingue ciò che Android annuncia da ciò che il contratto esegue.
Il riavvio automatico di una sequenza interrotta non è implementato: `session-inspect`
verifica integrità ma non dichiara né esegue un resume cieco.

### Verifiche successive completate

Evidenze: `Rust_App/target/release/captures/CLI_AUDIT_20260912_143339_623/results.json`.
**37 invocazioni, zero esiti inattesi**, incluse dieci scansioni d'integrità delle sessioni.

- Strict-results: exit 1 previsto, un frame conservato, errore hot_pixel/shading esplicito,
  sessione integra e ispezionabile. Il test ha usato capture singolo; il ramo di interruzione
  è condiviso con sequence, ma questo non va descritto come prova hardware di N frame interrotti.
- Massimo esposizione richiesto 16000001084 ns: uno scatto acquisito e verificato.
  Comando completo 50.422 s, inclusa anteprima finale lunga: non equivale alla sola esposizione.
- Sequenza 30x1s: 30/30 RAW, comando 59.849 s, esposizione osservata 999971248 ns
  su primo/ultimo frame, span timestamp sensore 54.708986088 s tra primo e ultimo.
  Tre campioni termici persistiti, tutti nominali, temperatura numerica sconosciuta.
- Tutti gli hash delle dieci sessioni prodotti dalla matrice verificati da session-inspect.
- Test workspace Rust con feature desktop passati; Android 15 unit test (6 protocollo,
  3 packing RAW, 6 validazione), zero failure/error; lint release e build release passati.
- Release CLI ricompilata; ultima release APK installata, servizio riavviato via azione ADB.

### Inventario completo e perimetro ancora NON implementato

`doc/camera-inventory-20260912.json` è il dump reale con ogni chiave Camera2 di richiesta,
rotta nell'adapter e chiavi risultato. Camera 0: **94 chiavi di richiesta, 27 mappate
a comportamenti dell'adapter, 67 non esposte**. Camera 1: 91, 26 mappate, 65 non esposte.
Le rotte comprendono anche impostazioni gestite automaticamente dall'adapter, non tutte
sono flag liberamente variabili. Non chiamare questa versione "ogni parametro controllabile".

Tra le chiavi pubbliche ancora non esposte: AE auto/compensazione/regioni/FPS range,
AWB lock, effetti/scena, post-RAW boost, flash, qualità/orientamento/thumbnail JPEG,
apertura/focale/filter density, statistiche opzionali e curve tonemap.
Tra le private Google: proxy camera token/ID, modalità processing interne e override termici.
Non tentare valori privati alla cieca, non aggirare protezioni termiche né spacciare
un elenco di chiavi per prova di controllo. Il passo successivo è aggiungere controlli
pubblici tipizzati con discovery/validazione/risultati; i limiti privati rimangono dichiarati.

Guida operativa e mappa: `doc/cli-control-reference.md`.

### Controprova strict su sequenza multipla

`STRICT_SEQUENCE_2026-09-12T124048Z_960620900`: richiesti 3 frame, primo frame
conservato, exit 1 per hot_pixel/shading forzati dall'HAL, 2 frame non avviati.
`session-inspect`: integrità true, saved=1, remaining=2. Conferma hardware
dell'interruzione, non soltanto del capture singolo.

Durante il lavoro sono apparsi README riorganizzato, AGENTS.md e asset/icona
Android/Rust da attività concorrente: preservati, non modificati da questa verifica.



last agents run: Leggo prima report.md, test.md, README e lo stato attuale del codice. Poi completo la catena CLI → app Kotlin → camera, distinguendo i controlli realmente supportati dal telefono da quelli indisponibili. Lavoro da solo, esclusivamente via shell/ADB, preservando le modifiche dell’altro agente.




  Ran commandsRan git status --short; rg --files -g '*report*' -g '*test.md' -g '*README*' -g 'AGENTS.md' -g 'Cargo.toml' -g '*cli*' -g '*Protocol*' -g '*Bridge*'




Ran Get-Content README.md; Get-Content report.md; Get-Content test.md




Ran Get-Content Rust_App/crates/deepsky-cli/src/main.rs; Get-Content Rust_App/crates/deepsky-cli/src/commands/acquire.rs; Get-Content Rust_App/crates/deepsky-cli/src/commands/preview.rs; rg --files Rust_App/crates/deepsky-camera/src Android_App/app/src/main/java/com/deepskyeyes/android; Get-Content report.md -Tail 100




Ran Get-Content Rust_App/crates/deepsky-camera/src/backend.rs; Get-Content Rust_App/crates/deepsky-camera/src/model.rs; Get-Content Android_App/app/src/main/java/com/deepskyeyes/android/transport/BridgeServer.kt; Get-Content Rust_App/crates/deepsky-cli/src/commands/mod.rs; rg -n 'struct CaptureSpec|struct AcquisitionOptions|fn build_request|CaptureSpec \{' Rust_App/crates/deepsky-app/src/controller.rs




Ran Get-Content Rust_App/crates/deepsky-app/src/controller.rs | Select-Object -Skip 270 -First 170; rg -n 'rpc|struct|fn ' Rust_App/crates/deepsky-app/src/remote.rs; Get-Content Android_App/app/src/main/java/com/deepskyeyes/android/camera/RequestValidator.kt; Get-Content Android_App/app/src/main/java/com/deepskyeyes/android/camera/CapabilityDiscovery.kt









Ho trovato due problemi concreti oltre ai parametri mancanti: la CLI parte sul simulatore se non si specifica la sorgente, e alcuni flag sconosciuti o il Kelvin possono essere ignorati. Correggo questi comportamenti prima di ampliare i controlli: un comando deve applicarsi davvero oppure fallire chiaramente.




Edited files, ran commandsRan Get-Content Rust_App/crates/deepsky-app/src/remote.rs -First 100; Get-Content Rust_App/crates/deepsky-app/src/controller.rs -First 110; rg -n 'CaptureSpec \{|AcquisitionOptions \{' Rust_App; Get-Content Rust_App/crates/deepsky-cli/Cargo.toml; Get-Content Android_App/app/src/main/AndroidManifest.xml; Get-Content Android_App/app/src/main/java/com/deepskyeyes/android/MainActivity.kt -First 90; rg -n 'white_balance_request|build_request\(' Rust_App/crates/deepsky-app/src/controller.rs




Ran Get-Content Rust_App/crates/deepsky-app/src/source.rs; Get-Content Rust_App/crates/deepsky-app/src/controller.rs | Select-Object -Skip 460 -First 35; Get-Content Rust_App/crates/deepsky-app/src/worker.rs | Select-Object -Skip 560 -First 28; Get-Content Rust_App/crates/deepsky-app/src/main.rs | Select-Object -Skip 100 -First 34; Get-Content Rust_App/crates/deepsky-camera/Cargo.toml; Get-Content Rust_App/crates/deepsky-protocol/src/rpc.rs -First 80; C:\platform-tools\adb.exe devices




Edited controller.rs+15-2




Edited worker.rs+1-0




Edited acquire.rs+2-1




Created controls.rs+47-0




Edited mod.rs+2-0




Edited preview.rs+1-0






Edited remote.rs+6-0




Edited backend.rs+4-0




Created diagnostics.rs+24-0




Edited main.rs+54-1






Ran cargo test -p deepsky-cli -p deepsky-app -p deepsky-camera -j 2




Ran rg -n 'fun capture|DngCreator|reported|EIS|eis|crop|fun buildRequest|fun autofocus' Android_App/app/src/main/java/com/deepskyeyes/android/camera/CameraEngine.kt; Get-Content Android_App/app/src/main/java/com/deepskyeyes/android/camera/CameraEngine.kt | Select-Object -Skip 155 -First 80; Get-Content README.md | Select-Object -Last 160; Get-Content Rust_App/crates/deepsky-cli/src/commands/camera.rs; Get-Content Rust_App/crates/deepsky-cli/src/commands/capabilities.rs




Edited MainActivity.kt+11-0




Edited CapabilityDiscovery.kt+8-1




Edited model.rs+1-0






Ran cargo build -p deepsky-cli --release -j 2




Ran & '.tools/gradle-9.6.0/bin/gradle.bat' :app:assembleRelease :app:testDebugUnitTest :app:lintRelease --no-daemon









Sono ora esposti crop, durata del frame, OIS, EIS e modalità di elaborazione, oltre ai comandi JSON per stato, temperatura, ping, dump completo e autofocus. Sto compilando anche Kotlin: aggiungo l’avvio del servizio da ADB e tolgo dall’elenco DNG le risoluzioni che il writer Android non può salvare; restano disponibili come RAW16.




Edited files, ran commandsRan Get-Content Rust_App/crates/deepsky-app/src/controller.rs | Select-Object -Skip 710 -First 175; Get-Content Android_App/app/src/main/java/com/deepskyeyes/android/camera/CameraEngine.kt | Select-Object -Skip 385 -First 52; Get-Content Android_App/app/src/test/java/com/deepskyeyes/android/RequestValidatorTest.kt -First 40; git diff --stat




Edited remote.rs+14-3




Edited diagnostics.rs+9-2






Ran $env:PATH = 'C:\platform-tools;' + $env:PATH; & Rust_App/target/release/deepsky-eyes.exe status




Ran Get-Content Rust_App/crates/deepsky-metadata/src/frame.rs -First 130; Get-Content Rust_App/crates/deepsky-app/src/controller.rs | Select-Object -Skip 875 -First 38; rg -n 'CameraCapabilities \{' Rust_App/crates --glob '*.rs'




Edited controller.rs+3-0






Ran C:\platform-tools\adb.exe -s 44101FDJG003S3 install -r -g Android_App/app/build/outputs/apk/release/app-release.apk; C:\platform-tools\adb.exe -s 44101FDJG003S3 shell am start -n com.deepskyeyes.android/.MainActivity -a com.deepskyeyes.android.START_BRIDGE




Ran cargo test --workspace --features deepsky-ui/desktop -j 2; if ($LASTEXITCODE -eq 0) { cargo build -p deepsky-cli --release -j 2 }




Ran $env:PATH = 'C:\platform-tools;' + $env:PATH; & Rust_App/target/release/deepsky-eyes.exe status; & Rust_App/target/release/deepsky-eyes.exe thermal; & Rust_App/target/release/deepsky-eyes.exe capabilities --camera 0




Ran Get-Content Rust_App/crates/deepsky-app/src/controller.rs | Select-Object -Skip 500 -First 125; Get-Content Rust_App/crates/deepsky-sequencer/src/validation.rs; Get-Content Android_App/app/src/main/java/com/deepskyeyes/android/camera/CameraEngine.kt | Select-Object -Skip 252 -First 44




Edited model.rs+2-0




Created execute.rs+50-0




Edited mod.rs+1-0




Edited main.rs+6-0






Ran cargo build -p deepsky-cli --release -j 2




Created cli-regression.ps1+45-0











L’APK aggiornato è installato e il servizio si avvia dal PC senza premere pulsanti nell’app. Ho aggiunto anche richieste JSON complete, per controlli avanzati e scatti JPEG separati dalle sequenze scientifiche RAW. Ora eseguo la matrice di prove reali, compresi gli errori intenzionali e la verifica che il servizio resti utilizzabile.




Edited a file, ran commandsRan & ./doc/cli-regression.ps1




Ran Get-Content Rust_App/crates/deepsky-protocol/src/codec.rs; $deepskyPid = (C:\platform-tools\adb.exe -s 44101FDJG003S3 shell pidof com.deepskyeyes.android).Trim(); C:\platform-tools\adb.exe -s 44101FDJG003S3 logcat -d --pid=$deepskyPid -s DSKY:D AndroidRuntime:E | Select-Object -Last 20; Get-Content Android_App/app/src/main/java/com/deepskyeyes/android/protocol/Json.kt




Edited remote.rs+1-1






Ran cargo build -p deepsky-cli --release -j 2; $env:PATH = 'C:\platform-tools;' + $env:PATH; & target/release/deepsky-eyes.exe capability_dump




Edited remote.rs+1-1






Ran cargo build -p deepsky-cli --release -j 2; $env:PATH = 'C:\platform-tools;' + $env:PATH; & target/release/deepsky-eyes.exe capability_dump









Il dump completo ha fatto emergere un bug reale dell’adapter: i numeri razionali di Camera2 venivano serializzati come 1/6, producendo JSON invalido. Lo correggo conservando numeratore e denominatore esatti; le prove stanno quindi verificando il percorso completo, non soltanto la compilazione.




Edited files, read files, ran commandsEdited CapabilityDiscovery.kt+4-1




Edited remote.rs+1-1






Ran & '.tools/gradle-9.6.0/bin/gradle.bat' :app:assembleRelease :app:testDebugUnitTest :app:lintRelease --no-daemon




Ran $env:PATH = 'C:\platform-tools;' + $env:PATH; & Rust_App/target/release/deepsky-eyes.exe autofocus --camera 0




Edited CapabilityDiscovery.kt+1-1




Edited main.rs+5-1






Ran Get-Content test.md -Tail 5; Get-Content report.md -Tail 5; rg --files doc | Select-Object -First 35; rg -n 'start_ns|thermal_events|record_thermal' Rust_App/crates/deepsky-session/src/store.rs




Read store.rs



Ran Get-Content Rust_App/crates/deepsky-session/src/recovery.rs -First 60; Get-Content Rust_App/crates/deepsky-app/src/controller.rs | Select-Object -Skip 625 -First 50




Edited controller.rs+7-2




Edited execute.rs+13-0




Edited main.rs+3-0






Ran cargo test -p deepsky-cli -p deepsky-app --lib --bins -j 2; if ($LASTEXITCODE -eq 0) { cargo build -p deepsky-cli --release -j 2 }




Ran C:\platform-tools\adb.exe -s 44101FDJG003S3 install -r -g Android_App/app/build/outputs/apk/release/app-release.apk; C:\platform-tools\adb.exe -s 44101FDJG003S3 shell am start -n com.deepskyeyes.android/.MainActivity -a com.deepskyeyes.android.START_BRIDGE




Ran & ./doc/cli-regression.ps1




Ran $auditDir = Get-ChildItem Rust_App/target/release/captures -Directory -Filter 'CLI_AUDIT_*' | Sort-Object Name | Select-Object -Last 1; Get-ChildItem -LiteralPath $auditDir.FullName -Recurse -Filter '*.dng.json' | Where-Object { $_.FullName -match 'CONTROLS|CROP' } | ForEach-Object { Get-Content -LiteralPath $_.FullName }; Get-Content doc/06-white-balance-color-processing.md -First 100









Le prove confermano OIS/EIS, durata del frame e diversi controlli di elaborazione. Hanno anche mostrato un limite dell’HAL: per questi RAW il Pixel riporta hot-pixel e shading su high_quality anche quando chiedo altro; il crop viene arrotondato di un pixel. Aggiungo avvisi espliciti e una modalità rigorosa che interrompe la sequenza quando i risultati non rispettano i controlli richiesti.




Edited files, ran commandsEdited controller.rs+35-1




Edited worker.rs+1-0




Edited acquire.rs+1-0




Edited main.rs+4-3






Ran cargo test -p deepsky-app -p deepsky-cli -j 2




Edited report.md+69-0




Edited test.md+27-0






Edited cli-regression.ps1+11-1






Ran cargo build -p deepsky-cli --release -j 2; if ($LASTEXITCODE -eq 0) { & ../doc/cli-regression.ps1 -Long }




Ran & '.tools/gradle-9.6.0/bin/gradle.bat' :app:assembleRelease :app:testDebugUnitTest :app:lintRelease --no-daemon




Ran Get-Content Rust_App/crates/deepsky-app/src/controller.rs | Select-Object -Last 70; Get-Content Rust_App/crates/deepsky-cli/src/main.rs | Select-Object -Skip 35 -First 45




Edited controller.rs+15-0




Edited main.rs+7-0






Ran Get-Content Android_App/app/src/main/java/com/deepskyeyes/android/camera/CapabilityDiscovery.kt | Select-Object -Skip 94 -First 25; Get-Content doc/cli-regression.ps1 -Tail 15




Edited CapabilityDiscovery.kt+37-1











La modalità rigorosa è stata verificata: conserva lo scatto e restituisce errore quando l’HAL forza parametri diversi. Anche lo scatto alla massima esposizione è passato; il comando completo, inclusa l’anteprima finale, ha richiesto 50,4 s. È in corso la sequenza di 30 scatti da 1 s, poi verifico tutti i file e i campioni termici.




Edited a file, ran commandsRan cargo test --workspace --features deepsky-ui/desktop -j 2




Ran & '.tools/gradle-9.6.0/bin/gradle.bat' :app:assembleRelease :app:testDebugUnitTest :app:lintRelease --no-daemon




Ran git diff --check; $auditDir = Get-ChildItem Rust_App/target/release/captures -Directory -Filter 'CLI_AUDIT_*' | Sort-Object Name | Select-Object -Last 1; $results = Get-Content -LiteralPath (Join-Path $auditDir.FullName 'results.json') -Raw | ConvertFrom-Json; $results | Where-Object { $_.name -eq 'strict-results-preserve-and-stop' -or $_.name -eq 'maximum-exposure' } | Select-Object name,output | Format-List; Get-ChildItem -LiteralPath $auditDir.FullName -Directory -Filter 'STABILITY*' | ForEach-Object { (Get-ChildItem -LiteralPath (Join-Path $_.FullName 'lights') -Filter '*.dng').Count }




Ran $auditDir = Get-ChildItem Rust_App/target/release/captures -Directory -Filter 'CLI_AUDIT_*' | Sort-Object Name | Select-Object -Last 1; $seqDir = Get-ChildItem -LiteralPath $auditDir.FullName -Directory -Filter 'STABILITY*' | Select-Object -First 1; $revision = Get-ChildItem -LiteralPath (Join-Path $seqDir.FullName 'metadata') -Filter 'session-*.json' | Sort-Object Name | Select-Object -Last 1; $manifest = Get-Content -LiteralPath $revision.FullName -Raw | ConvertFrom-Json; [pscustomobject]@{Audit=$auditDir.Name;Completed=$manifest.frames_completed;ThermalSamples=$manifest.thermal_events.Count;FirstExposure=$manifest.frames[0].metadata.exposure_reported_ns;LastExposure=$manifest.frames[-1].metadata.exposure_reported_ns;SensorSpanSeconds=($manifest.frames[-1].timestamp_ns-$manifest.frames[0].timestamp_ns)/1e9}; $manifest.thermal_events | ConvertTo-Json




Created cli-control-reference.md+102-0











La sequenza ha completato 30/30 RAW, tutti verificati tramite checksum, con tre campioni termici nominali salvati. Ogni esposizione riporta 0,999971 s; il tempo complessivo è stato 59,85 s. Sto completando la documentazione della CLI e l’inventario dei controlli, indicando anche quelli nativi che l’adapter non espone.




Edited files, ran commandsRan cargo build -p deepsky-cli --release -j 2




Ran C:\platform-tools\adb.exe -s 44101FDJG003S3 install -r -g Android_App/app/build/outputs/apk/release/app-release.apk; C:\platform-tools\adb.exe -s 44101FDJG003S3 shell am start -n com.deepskyeyes.android/.MainActivity -a com.deepskyeyes.android.START_BRIDGE




Ran $auditDir = Get-ChildItem Rust_App/target/release/captures -Directory -Filter 'CLI_AUDIT_*' | Sort-Object Name | Select-Object -Last 1; $results = Get-Content -LiteralPath (Join-Path $auditDir.FullName 'results.json') -Raw | ConvertFrom-Json; [pscustomobject]@{Tests=$results.Count;Failures=@($results | Where-Object {$_.actual -ne $_.expected}).Count}; Get-ChildItem Android_App/app/build/test-results/testDebugUnitTest -Filter 'TEST-*.xml' | ForEach-Object { [xml]$r = Get-Content -LiteralPath $_.FullName; [pscustomobject]@{Suite=$r.testsuite.name;Tests=$r.testsuite.tests;Failures=$r.testsuite.failures;Errors=$r.testsuite.errors} } | Format-Table




Ran $env:PATH = 'C:\platform-tools;' + $env:PATH; $dumpText = & Rust_App/target/release/deepsky-eyes.exe capability_dump --out doc/camera-inventory-20260912.json; if ($LASTEXITCODE -ne 0) { throw 'dump failed' }; $dump = $dumpText -join "`n" | ConvertFrom-Json; $dump.cameras | ForEach-Object { [pscustomobject]@{Camera=$_.camera_id;Characteristics=$_.characteristics.PSObject.Properties.Count;RequestKeys=$_.request_controls.Count;Mapped=@($_.request_controls | Where-Object exposed).Count;Unmapped=@($_.request_controls | Where-Object {-not $_.exposed}).Count} } | Format-List; & Rust_App/target/release/deepsky-eyes.exe status; & Rust_App/target/release/deepsky-eyes.exe --source adb --realtime discover




Ran $inventory = Get-Content doc/camera-inventory-20260912.json -Raw | ConvertFrom-Json; $inventory.cameras[0].request_controls | Where-Object {-not $_.exposed} | Select-Object -ExpandProperty key; rg -n 'requestMetadata|captureRequest|whiteBalance|native|temperature|colorTemperature|colorTint' doc/camera-inventory-20260912.json | Select-Object -First 30




Ran rg -n 'CaptureSettings \{' Rust_App --glob '*.rs'; Get-Content Android_App/app/src/main/java/com/deepskyeyes/android/camera/CameraEngine.kt -First 70; Get-Content Android_App/app/src/test/java/com/deepskyeyes/android/ValidatorTest.kt -First 30




Edited report.md+36-3




Edited test.md+7-3






Ran git diff --check; git status --short; Get-Item Rust_App/target/release/deepsky-eyes.exe, Android_App/app/build/outputs/apk/release/app-release.apk | Select-Object FullName,Length,LastWriteTime




Ran Get-Content AGENTS.md; git diff -- README.md




Ran Get-Content AGENTS.md | Select-Object -Skip 600 -First 1000




Ran Get-Content AGENTS.md | Select-Object -Skip 1600 -First 1200




Ran Get-Content AGENTS.md | Select-Object -Skip 2800




Ran $env:PATH = 'C:\platform-tools;' + $env:PATH; & Rust_App/target/release/deepsky-eyes.exe sequence --camera 0 --project STRICT_SEQUENCE --frames 3 --exposure-ns 100000000 --sensitivity 100 --wb-preset daylight --processing hot_pixel=off,shading=off --strict-results; $strictExit = $LASTEXITCODE; $strictDir = Get-ChildItem Rust_App/target/release/captures -Directory -Filter 'STRICT_SEQUENCE_*' | Sort-Object Name | Select-Object -Last 1; $scanText = & Rust_App/target/release/deepsky-eyes.exe session-inspect --path $strictDir.FullName; $scanResult = $scanText -join "`n" | ConvertFrom-Json; [pscustomobject]@{Exit=$strictExit;Requested=$scanResult.manifest.frames_requested;Saved=$scanResult.manifest.frames_completed;Integrity=$scanResult.integrity_ok;Remaining=$scanResult.frames_remaining;Session=$strictDir.FullName}; if ($strictExit -ne 1 -or $scanResult.manifest.frames_completed -ne 1 -or -not $scanResult.integrity_ok) { throw 'strict sequence regression failed' }




Edited report.md+10-0




Edited test.md+1-1














CLI e Kotlin aggiornati, release compilate e APK installato.
Verificati via ADB: 37 prove, 30/30 RAW con checksum, autofocus, controlli avanzati e arresto rigoroso sulle discrepanze dell’HAL.
Il controllo totale non è ancora raggiunto: l’inventario distingue tutti i parametri, ma restano controlli pubblici da implementare e chiavi private Google non utilizzabili alla cieca.
Risultati e lavoro restante in [report.md](E:/project-seri/DeepskyEyes/report.md), [test.md](E:/project-seri/DeepskyEyes/test.md) e [guida CLI](E:/project-seri/DeepskyEyes/doc/cli-control-reference.md).

## Appendice di fermo — 2026-09-12

- Su richiesta dell'utente, lavoro fermato. In questa continuazione ho solo verificato worktree/ADB e letto modello Rust e parti Kotlin: **nessuna nuova implementazione, build o acquisizione**.
- Stavo preparando i controlli JPEG tipizzati (qualità, orientamento, qualità/dimensioni thumbnail), da esporre via CLI JSON con discovery, validazione e risultati reali. Non ancora implementati.
- Rimane valido il lavoro precedente: CLI ampliata, APK release installato, 37 prove con esito atteso, 30/30 RAW verificati, autofocus e strict-results testati. Non significa controllo universale: camera 0 ha 94 chiavi, 27 mappate e 67 non esposte nell'inventario attuale.
- Aggiunto in fondo a test.md il piano completo di ripresa: JPEG, altri controlli pubblici, regressioni, qualità RAW, guasti/recovery e prove lunghe. Le attività non eseguite restano esplicitamente da fare.
- Preservati lavoro concorrente e file esistenti. Nessuna attività nuova lasciata in esecuzione da questa continuazione. Attendere indicazione dell'utente prima di riprendere.
