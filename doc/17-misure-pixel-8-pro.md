# 17 — Misure reali Pixel 8 Pro (E2E 2026-09-11, ADB → Rust)

Prima luce: Rust `deepsky-eyes` contro Kotlin `BridgeServer` su Pixel 8 Pro / Android 17.
Niente di quanto segue è assunto: tutto è stato annunciato dal device o misurato.

## 1. Discovery

- Camera `0`: back, logical, RAW ✓, manual_sensor ✓, 23 stream, fisiche [2,3,4,5,6] (non apribili direttamente)
- Camera `1`: front, logical, RAW ✓, manual ✓, 12 stream, fisiche [7,8]

## 2. Capability camera 0 (misurate, non assunte)

| Parametro | Annunciato |
|---|---|
| exposure_ns | 26345 … 16000001084 (~16.0 s max → 15 s fattibile, 20 s clamperà) |
| sensitivity | 21 … 10666 |
| frame_duration_ns | min 0 (nessun minimo dichiarato) … 16000001084 |
| focus | 0 … 9523 millidiopters, AF OFF ✓, focus lock ✓, calibrazione `1` |
| zoom_x1000 | 495 … 30000 (0.5x … 30x) |
| active_array | 4080 × 3072 |
| RAW | 4080×3072 Dng + Raw16Le, min_frame 33 ms, stall 0 |
| hardware_level | `1` |
| lens_role | null (Camera2 non dà etichette wide/tele: mapping da validare) |

## 3. Scatto reale (1 s, ISO 100, fuoco 0 = infinito, WB daylight di fallback)

- DNG 4080×3072 da 25.107.216 byte, magic `49 49 2A 00` (TIFF LE valido)
- Richiesto 1.000.000.000 ns → riportato 999.971.248 ns (differenza registrata, non nascosta)
- Fisica attiva riportata: `2` (sensore main), OIS off, zoom 1.0, crop full-sensor
- Processing: edge/noise/aberration/distortion OFF come richiesto; hot_pixel/shading/tonemap HIGH_QUALITY imposti dal device (non annunciava "off")
- Sequenza 3/3 frame committed, sha256 registrati; preview Gray8 dal piano Y con luma media misurata 92.32

## 4. Bug trovati dall'hardware (già fixati)

1. `frame_duration_ns.min == 0` veniva rifiutato come capability invalida → ora ammesso ed esplicitamente documentato (nessun minimo dichiarato ≠ capability rotta). File: `deepsky-camera/src/model.rs`.

## 5. Da misurare ancora (serve sessione lunga sul campo)

15 s × N stabilità esposizione, deriva termica/noise-vs-temp, OIS on/off su RAW,
ripetibilità fuoco, 12MP binned vs 50MP full, dark library per ISO/tempo/temperatura.
