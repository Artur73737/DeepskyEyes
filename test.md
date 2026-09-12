# DeepskyEyes — Piano Test Controllo Totale Pixel 8 Pro via ADB

> Obiettivo: l'exe Rust deve poter controllare **ogni parametro** annunciato dal Pixel. Ogni cosa che il telefono mette a disposizione deve funzionare, oppure deve fallire in modo esplicito (mai silenzioso).
> Filosofia: Discover → Validate → Capture → Record. `Reject`, mai clamp silenzioso.

Telefono: `44101FDJG003S3` — `Google Pixel 8 Pro / Android 17`
Exe: `Rust_App/target/release/deepsky-eyes.exe`
Sorgente: `--source adb --serial 44101FDJG003S3`
App Kotlin: `com.deepskyeyes.android` avviata, service `127.0.0.1:7878` via ADB forward.

Riferimento capabilities misurate il 2026-09-12 (camera 0 back, camera 1 front):

| Parametro | Cam 0 (back) | Cam 1 (front) |
|---|---|---|
| `exposure_ns` | 26345 .. 16000001084 (~16s) | 46377 .. 1000003181 (~1s) |
| `sensitivity` | 21 .. 10666 | 51 .. 4873 |
| `focus_millidiopters` | 0 .. 9523, `af=[off]`, lock=true | 0 .. 10000, `af=[off]` |
| `zoom_x1000` | 495 .. 30000 (0.5x..30x) | 896 .. 10000 |
| `streams RAW` | Dng+Raw16Le 4080x3072, 4080x2288, 2032x1536, 2016x1136 | Dng+Raw16Le 3440x2448 |
| `streams JPEG/preview` | Jpeg x13 + Rgb8 preview ≤1280x720 | Jpeg x10 + preview |
| `wb_modes` | auto, incandescent, fluorescent, warm_fluorescent, daylight, cloudy_daylight, twilight, shade (`wb_kelvin=null`) | = |
| `processing` | aberration[off], distortion[off,fast], edge[off,fast,high_quality], hot_pixel[off,fast,high_quality], noise_reduction[off,fast,high_quality], shading[off,fast,high_quality], tonemap[fast,high_quality] | = |
| `ois/eis` | [off,on] / [off,on] | = |
| `logical/physical` | logical=true, phys 2,3,4,5,6 non apribili | logical=true, phys 7,8 |
| `manual/raw` | true / true, hw level 1 | true / true |

Legenda: `[ ]` da fare · `[x]` pass · `[!]` fail/bug · `[~]` limitazione by-design

---

## T0 — Setup e connessione [x]
- [x] T0.1 `adb devices` vede `44101FDJG003S3 device`
- [x] T0.2 app `com.deepskyeyes.android/.MainActivity` in foreground
- [x] T0.3 `deepsky-eyes --source adb --serial ... discover` → `0 back + 1 front`
- [ ] T0.4 `adb forward --list` verifica forward `tcp:XXXX → tcp:7878` creato/rimosso
- [ ] T0.5 `ping` / `GET_VERSION` via Rust (se esposto) o logcat `Listening on 127.0.0.1:7878`

## T1 — Discovery / capabilities
- [x] T1.1 `camera list` → 2 camere, origin=Device
- [x] T1.2 `capabilities --camera 0` → JSON schema_version=1, 23 streams
- [x] T1.3 `capabilities --camera 1` → 12 streams, max exp ~1s (verificato 2026-09-12: exp 46377..1000003181, iso 51..4873)
- [x] T1.4 `capabilities --camera 99` → deve fallire `not announced` (verificato EXIT:1)
- [ ] T1.5 `capability_dump` esteso (tutte le `CameraCharacteristics`) salvato in `doc/`
- [ ] T1.6 verifica physical `2,3,4,5,6` → `directly_openable=false`, open diretto deve fallire
- [ ] T1.7 verifica `lens_facing back/front`, `hardware_level`, `active_array`, `crop_supported=true`

## T2 — Esposizione (`SENSOR_EXPOSURE_TIME`)
- [x] T2.1 `capture --exposure-ns 1000000000` (1s) → req≈rep (visto: 1000000000→999971248)
- [ ] T2.2 minimo `261..26345ns` → deve riuscire o `OutOfRange` esplicito
- [x] T2.3 `100000000` (0.1s) indoor → preview non bruciata (verificato: preview 640x480 luma 38.0 + DNG 1/1)
- [ ] T2.4 massimo cam0 `16000001084` (~16s, `--realtime`? timeout 120s) → 1 scatto
- [ ] T2.5 oltre max `30000000000` → deve fallire `Reject/OutOfRange`, MAI clamp silenzioso
- [ ] T2.6 `0` → deve fallire `ValidationFailed(empty plan)`
- [ ] T2.7 coerenza req/rep su 3 scatti identici (stabilità)

## T3 — Sensibilità / ISO (`SENSOR_SENSITIVITY`)
- [x] T3.1 `800` → req 800 → rep 799 (tolleranza ±2, visto ok)
- [x] T3.2 minimo cam0 `21` → 1 scatto (verificato T15_ISOMIN)
- [x] T3.3 massimo cam0 `10666` → 1 scatto (verificato T15_ISOMAX)
- [ ] T3.4 oltre max `20000` → fail esplicito
- [ ] T3.5 sotto min `10` → fail esplicito
- [ ] T3.6 `ISO auto` → NON esiste (solo range) → documentare, nessun flag auto nel CLI
- [ ] T3.7 stessa scena a ISO 100/800/6400 → DNG + metadati confrontati

## T4 — Fuoco (`LENS_FOCUS_DISTANCE`, `CONTROL_AF_MODE`)
- [x] T4.1 `manual 0 mdiopt locked` (infinito) → ok, `focus_mode=manual`
- [ ] T4.2 `4761` (medio) → req/rep verificati
- [ ] T4.3 `9523` (max cam0) → 1 scatto
- [ ] T4.4 oltre max `20000` → fail `Reject`
- [ ] T4.5 `preview --autofocus` → SORPRESA 2026-09-12: RIESCE (`Center AF: 508 mdiopt`). La vecchia attesa `Unsupported` era sbagliata: l'HAL supporta AF_AUTO, il modello caps lo nasconde. Da aggiornare `CapabilityDiscovery` o documentare workflow AF→manual
- [ ] T4.6 sequenza senza `locked` → DEVE fallire `focus must be locked` (validazione §36)

## T5 — Bilanciamento bianco (`CONTROL_AWB_MODE`, gains)
- [x] T5.1 `daylight` fisso → ok (10x1s del 2026-09-12)
- [ ] T5.2 ognuno: `incandescent, fluorescent, warm_fluorescent, cloudy_daylight, twilight, shade` → 1 scatto cad., `white_balance_mode` riportato
- [ ] T5.3 `auto` → DEVE fallire `fixed white balance required` (già visto il 2026-09-12, by-design astro)
- [ ] T5.4 `wb-kelvin 5000` senza preset → verificare che venga ignorato / mappato su preset (è `null` sul device, nessun modo `temperature`)
- [ ] T5.5 preset inesistente `--wb-preset alien` → fail `Unsupported`

## T6 — Zoom / Crop (`CONTROL_ZOOM_RATIO`, `SCALER_CROP_REGION`)
- [x] T6.1 zoom 1.0x (`1000`) → ⚠️ RISOLTO: ora `--zoom RATIO` e `--zoom-x1000 N` (verificato: `--zoom 2.0` → req 2.0 rep 2.0, 1 DNG)
- [x] T6.2 zoom min `495`, max `30000` → `--zoom 30.1` oltre max → fail `OutOfRange: zoom_x1000` ✓
- [x] T6.3 `--zoom 2.0 --zoom-x1000 2000` insieme → fail `config: --zoom and --zoom-x1000 are exclusive` ✓
- [ ] T6.4 crop full `0,0,4080,3072` riportato (visto) → crop custom se esposto in futuro

## T7 — Stream / Risoluzione (RAW binned vs full, JPEG, preview)
- [x] T7.1 default `Dng 4080x3072` (primo stream RAW) → 24MB/cad. ok
- [x] T7.2 `Raw16Le 4080x3072` → RISOLTO: `--stream 4080x3072:Raw16Le` → 1×`.raw16` full ok
- [!] T7.3 `2032x1536 Dng` → **BUG-001**: `DngCreator.nativeWriteImage` → `AssertionError` → morte processo `com.deepskyeyes.android` (log 14:14:22). Fix applicato: `CameraEngine` mappa a `Unsupported(use Raw16Le…)` + `BridgeServer` cattura `Throwable`. APK rebuild+install 14:16, da riverificare dopo restart service
- [ ] T7.4 JPEG / preview-only (`raw=false`) → CLI non espone → GAP residuo (capture resta RAW-only by-design, errore esplicito `not a RAW format`)
- [x] T7.5 `--stream 9999x9999:Dng` → da riverificare (test precedente inquinato dal crash BUG-001: `NotConnected` a server morto)

## T8 — RAW / DNG scientifico
- [x] T8.1 DNG apribile (Siril/LR), 25107216 byte, sha256 in manifest
- [ ] T8.2 verifica `CFA, black/white level, color matrix` dentro DNG (exiftool/dcraw)
- [ ] T8.3 confronto `Dng` (Android) vs `Raw16Le` diretto (se sbloccato T7.2)
- [ ] T8.4 linearità/noise a 1s ISO800 su 10 frame (mean/median/varianza)

## T9 — Preview (`YUV_420_888` → PNG)
- [x] T9.1 `preview --camera 0` → PNG + `mean luma` (visto 244 indoor bruciato; 38.0 a 100ms ISO100)
- [x] T9.6 preview a `100ms ISO100` → immagine non bruciata indoor (verificato 2026-09-12, stanza visibile)
- [x] T9.2 `preview --camera 1` (front) → deve riuscire (verificato: 640x480 luma 19.2)
- [ ] T9.3 `preview --camera 99` → fail `not announced`
- [ ] T9.4 `preview --frames 5` → 5 frame, `sensor_delta` coerenti
- [x] T9.5 `preview --autofocus` → RIESCE (vedi T14.2, 508 mdiopt)
- [ ] T9.6 preview a `100ms ISO100` → immagine non bruciata indoor

## T10 — Processing / OIS / EIS
- [x] T10.1 default riportati: `aberration off, distortion off, edge off, hot_pixel high_quality, noise_reduction off, shading high_quality, tonemap fast, ois off` → ⚠️ CLI NON espone `--processing/--ois/--eis`, usa `off` dove possibile → **GAP**
- [ ] T10.2 `ois on / eis on` → da implementare per test mosso astronomico
- [ ] T10.3 `noise_reduction off` forzato → verificare su DNG (dettaglio §19 README)

## T11 — Sequencer / Sessioni / Storage
- [x] T11.1 `sequence --frames 10` → `frames_committed:10`, `session.json`, sha256/frame
- [x] T11.2 `capture` singolo (frames=1) (verificato TEXP 100ms ISO100 → 1 DNG + preview)
- [x] T11.3 `--kind dark/flat/bias/test/light` → cartelle `darks/flats/bias/test/lights` + `frame_filename` deterministico (verificati `dark` e `test`)
- [ ] T11.4 `--delay-ns 0` vs `1000000000` → timing inter-frame misurato
- [ ] T11.5 `--project` con spazi/simboli → sanitizzato (`_` visto in `prepare_run`)
- [x] T11.6 `--frames 0` → fail `frames must be positive` (verificato)
- [ ] T11.7 crash-recovery: kill dopo N frame → `scan/can_resume`, nessun overwrite
- [ ] T11.8 `session.json` contiene `capability_snapshot, thermal_events, warnings, errors`

## T12 — Errori / Robustezza (failure-injection manuale)
- [x] T12.1 WB auto rifiutato (visto)
- [x] T12.1b camera inesistente rifiutata `not announced` (visto cam 99)
- [ ] T12.2 stacca USB durante sequenza → `PAUSED/FAILED` + reconnect sicuro, mai resume cieco
- [ ] T12.3 chiudi app Android durante sequenza → `DISCONNECTED` esplicito
- [ ] T12.4 disco pieno / path non scrivibile → `STORAGE_ERROR`
- [ ] T12.5 screen-off / Doze durante 10x1s → la `CameraService` sopravvive?
- [ ] T12.6 termico: 30x1s continui → `severity/temperature_c` in manifest

## T14 — Modalità AUTOMATICHE scelte dalla camera (AE / AF / AWB / ISO auto / tutto-auto)
> Scopo: verificare se l'exe lascia decidere alla camera (HAL) oppure impone valori espliciti.
> Atteso by-design (README §14-18, §36): l'exe DEVE rifiutare gli auto per astrofoto deterministica.
> Ogni test AUTO deve quindi fallire con codice macchina esplicito, oppure risultare come GAP (flag inesistente).
- [x] T14.1 `AWB auto` → `capture --wb-preset auto` → DEVE fallire `ValidationFailed(fixed white balance required)` (verificato 2 volte)
- [x] T14.2 `AF auto` → `preview --autofocus` → SORPRESA: RIESCE! `Center AF: 508 millidiopters`, preview ok luma 63.5. L'HAL ha AF_AUTO anche se `caps.af_modes=[off]` (filtrate). Workflow corretto: AF una volta → riusa valore come `manual` locked
- [x] T14.3 `AE auto (esposizione automatica)` → GAP: CLI non ha flag `--exposure auto`, `exposure_ns` è sempre `u64` obbligatorio → documentare
- [x] T14.4 `ISO auto` → GAP: CLI non ha flag `--sensitivity auto`, range 21..10666 ma nessun modo auto annunciato → documentare
- [x] T14.5 `Kelvin auto / temperature auto` → GAP by-device: `wb_kelvin=null` sempre, nessun modo `temperature` annunciato → `--wb-kelvin` ignorato
- [x] T14.6 `tutto-auto` (exp+iso+af+awb tutti auto in un colpo) → impossibile: nessun flag auto esiste, sequenza non parte senza valori espliciti
- [x] T14.7 `CONTROL_MODE AUTO del HAL` → l'engine Android imposta sempre `AE OFF + AF OFF + AWB preset fisso` in `buildRequest()` → verificare su logcat `DSKY`

## T15 — CUSTOM settings matrix (ogni parametro impostato a mano, req vs rep)
> Tutti con `--exposure-ns` corto (100ms) per velocità, salvo T15.1. Out: `foto_test/`.
- [x] T15.1 EXP custom 1s ISO800 daylight focus0 → riferimento (già fatto 10x1s)
- [x] T15.2 EXP min `26345` → ok, 1 DNG luma 0.9
- [x] T15.3 EXP oltre max `30000000000` → fail `Io: OutOfRange: exposure_ns`
- [x] T15.4 ISO min `21` → 1 DNG ok (21→21)
- [x] T15.5 ISO max `10666` → 1 DNG ok (10666→10664)
- [x] T15.6 ISO oltre max `20000` → fail `Io: OutOfRange: sensitivity`
- [x] T15.7 FOCUS medio `4761` → ok (4.761→4.775, tolleranza HAL)
- [x] T15.8 FOCUS max `9523` → 1 DNG ok
- [x] T15.9 FOCUS oltre max `20000` → fail `Io: OutOfRange: focus_millidiopters`
- [x] T15.10 WB `shade` → `white_balance_mode=shade` riportato
- [x] T15.11 WB `twilight` → ok; restano `cloudy_daylight, incandescent, fluorescent, warm_fluorescent` (stesso codice, basso rischio)
- [x] T15.12 WB preset fantasma `--wb-preset alien` → fail `Unsupported: white-balance preset not announced`
- [x] T15.13 KIND `dark` → cartella `darks/` ok; `test` → `test/` ok (verificati T15_EXPMIN/FOCUSMAX/WBTWI/DARK)
- [x] T15.14 FRAMES `0` → fail `config: frames must be positive`

## T13 — GAP CLI emersi (da implementare per “controllo su ogni parametro”)
- [x] T13.1 `capture/sequence` ora accettano `--camera ID` (verificato: `--camera 1` → DNG front 3440x2448, exp 100000000→99993645)
- [x] T13.2 `--zoom`, `--stream` implementati (`controller::parse_zoom/resolve_stream`, `--stream WxH:FORMAT[:MODE]`); restano `--crop`, `--ois`, `--eis`, `--processing`
- [ ] T13.3 nessun `--frame-duration-ns` esplicito
- [ ] T13.4 nessun comando `autofocus` diretto (solo `preview --autofocus`, che deve comunque fallire su Pixel per T4.5)
- [ ] T13.5 `serve`/`discover` ok, mancano `thermal`, `status`, `capability_dump` da CLI

---

## Come eseguire (template)

```bash
EXE="E:/project-seri/DeepskyEyes/Rust_App/target/release/deepsky-eyes.exe"
ADB="--source adb --serial 44101FDJG003S3"
$EXE $ADB discover
$EXE $ADB capabilities --camera 0 > doc/cap0.json
$EXE $ADB capabilities --camera 1 > doc/cap1.json
$EXE $ADB camera list
$EXE $ADB preview --camera 0 --exposure-ns 100000000 --sensitivity 100 --wb-preset daylight --out /tmp/p0.png
$EXE $ADB capture --out ./foto_test --project TEXP --exposure-ns 1000000000 --sensitivity 800 --focus-mdiopt 0 --wb-preset daylight
$EXE $ADB sequence --out ./foto_test --project TSEQ --frames 3 --exposure-ns 1000000000 --sensitivity 800 --wb-preset daylight --delay-ns 0
```

Esito atteso per ogni test: `EXIT:0 + frames_committed` oppure `EXIT:1 + codice macchina` (`OutOfRange`, `Unsupported`, `ValidationFailed`, …). Mai `ok` con valori clampati di nascosto: controllare sempre `requested vs reported` nel `.dng.json`.

## T16 — Continuazione CLI/Kotlin 2026-09-12

Le caselle storiche sopra non sono riscritte: questo aggiornamento prevale sulle vecchie
affermazioni (Kelvin ignorato, AF atteso Unsupported, DNG binnato annunciato).

- [x] Servizio avviato dal PC con azione START_BRIDGE, senza tap nell'app.
- [x] CLI default ADB; simulatore opt-in; test parser contro typo/duplicati/valori mancanti.
- [x] `status`, `ping`, `thermal`, `capability_dump` reali.
- [x] Dump valido dopo correzione Rational, conservazione esatta numeratore/denominatore.
- [x] Autofocus standalone: configurazione esplicita, AF centrale, distanza restituita in JSON.
- [x] DNG 2032x1536 rifiutato prima dello scatto; servizio ancora vivo.
- [x] RAW16 2032x1536 acquisito realmente; dimensioni non sostituite.
- [x] Risoluzione inesistente, processing inesistente, crop fuori array, durata troppo corta, Kelvin non supportato: exit 1.
- [x] OIS/EIS on e frame duration 200ms osservati nel CaptureResult.
- [x] Crop personalizzato applicato con arrotondamento di un pixel registrato nei metadati.
- [x] Processing: edge/NR/distortion fast e tonemap high_quality rispettati.
- [~] Processing: hot_pixel/shading restano high_quality sull'HAL per questo RAW, anche richiesti fast/off.
- [x] WB incandescent, fluorescent, warm_fluorescent, cloudy_daylight acquisiti.
- [x] Cinque preview consecutive; JPEG reale via template JSON + execute.
- [x] Execute rifiuta di sovrascrivere file esistenti.
- [x] Strict-results: exit 1 dopo frame conservato (capture singolo; interruzione sequenza multipla da verificare separatamente).
- [x] Sequenza 30x1s: 30/30, tre campioni termici nominali, tutti gli hash verificati.
- [x] Massimo esposizione: scatto riuscito; comando con preview finale 50.422s.
- [x] Ultima matrice: 37 invocazioni con exit atteso, evidenze CLI_AUDIT_20260912_143339_623/results.json.
- [x] Inventario completo delle chiavi native salvato in doc/camera-inventory-20260912.json.
- [ ] Esposizione CLI dei rimanenti controlli pubblici Camera2 tipizzati: vedere fine report.md.
- [~] Chiavi private Google elencate ma non scritte alla cieca; nessun bypass termico.

Evidenze della prima matrice: `Rust_App/target/release/captures/CLI_AUDIT_20260912_143032_129/results.json`.
Ripetizione: `powershell -File doc/cli-regression.ps1` (produce fotografie vere in una nuova cartella).
