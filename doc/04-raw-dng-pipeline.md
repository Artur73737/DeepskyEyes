# 04 — RAW / DNG pipeline (ricerca online)

Copre: README §12, §13, §52, §93.

## 1. Path ufficiale Android

```
Camera2 → ImageFormat.RAW_SENSOR → ImageReader → CaptureResult → DngCreator → .dng
```

- `DngCreator(CameraCharacteristics, CaptureResult)` genera tag DNG dai metadati di cattura.
- `writeImage(OutputStream, Image)` / `writeInputStream` / `writeByteBuffer`: richiedono `RAW_SENSOR`, 16 bit/pixel, `offset + 2*w*h` byte. Senza metadati sufficienti → `IllegalStateException`.
- Riferimento DNG: Adobe DNG 1.4.0.0. Sample ufficiale: `android-Camera2Raw`.
- CTS `DngCreatorTest`: scatta RAW per ogni camera e salva DNG; per ultra-high-res usa il primo size RAW max-resolution.
- Consiglio AOSP: per DNG di qualità abilitare lens shading map output.

## 2. Strategia DeepskyEyes (§13) — A/B/C

- **A — DNG su Android:** interoperabile, metadati Android, subito usabile. Contro: transfer più grande, costo CPU/telefono, meno controllo.
- **B — RAW_SENSOR diretto su PC:** massimo controllo, processing tutto su PC, potenzialmente più efficiente. Contro: bisogna trasportare tutti i metadati sensore e scrivere DNG in Rust.
- **C — entrambi:** utile in fase diagnostica (`raw packet` + `DNG`).

Filosofia: preservare la rappresentazione **meno processata possibile**.

## 3. Validità scientifica (§52) — cosa verificare

Bayer pattern, bit depth, black/white/saturation level, guadagni analogico/digitale,
read noise, dark current, FPN, hot/defective pixel, dipendenza termica, lens shading,
optical black, crop/binning/remosaic/pixel mode. La capability RAW garantisce `RAW_SENSOR` +
metadati DNG opzionali, **non** garantisce comportamento da astro-cam: serve validazione empirica (§07).

## 4. Checklist RAW (§93)

RAW_SENSOR disponibile? dimensioni esatte, bit depth, Bayer, black/white/saturation,
completezza metadati, correttezza DNG, fattibilità RAW diretto, confronto DNG Android vs DNG desktop.

## Fonti

- https://developer.android.com/reference/android/hardware/camera2/DngCreator
- https://github.com/googlearchive/android-Camera2Raw
- https://android.googlesource.com/platform/frameworks/base/+/master/core/java/android/hardware/camera2/DngCreator.java
- https://android.googlesource.com/platform/cts/+/master/tests/camera/src/android/hardware/camera2/cts/DngCreatorTest.java
- https://developer.android.com/reference/android/graphics/ImageFormat
