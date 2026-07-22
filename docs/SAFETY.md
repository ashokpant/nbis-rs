# Memory safety and FFI invariants

`nbis-python` combines Rust (UniFFI), NBIS C, NFIQ2 C++, and OpenCV. This document lists the guarantees the Rust layer enforces and what embedders must still provide.

## Platform

- Linux wheels: `manylinux_2_28_x86_64` and `manylinux_2_28_aarch64` (glibc ≥ 2.28). **Ubuntu 24.04** hosts and containers are supported on both architectures.
- **Linux wheels (0.1.13+)**: OpenCV **4.13** is vendored in the wheel; `libnbis.so` uses `$ORIGIN` RPATH — no host OpenCV install required.
- **macOS / custom builds**: OpenCV **4.13** shared libs on the loader path or `libnbis.so` RPATH — see [DEPENDENCIES.md](../DEPENDENCIES.md).
- Do **not** set global `LD_LIBRARY_PATH` to OpenCV when also using `opencv-python` (`import cv2`).

## Ownership (who frees what)

| Resource | Owner | Freed by |
|----------|--------|----------|
| `get_minutiae` maps / binarized image | Rust `MindtctOutputs` guard | `Drop` → `free` / `free_minutiae` |
| `get_minutiae` minutiae list | Same guard | `free_minutiae` |
| NFIQ2 result strings/arrays | C `nfiq2wrapper_compute` | `nfiq2wrapper_free_results` (always, including errors) |
| NFIQ2 wrapper instance | `Nfiq2` | `Drop` → `nfiq2wrapper_destroy` |
| SIVV CSV string | `sivv_ffi_from_bytes` | `sivv_ffi_free_bytes` after copy to Rust `String` |

## Threading

- **Single process**: `extract_minutiae` and `Minutiae::compare` are serialized by an internal `NBIS_NATIVE_MUTEX` (mindtct, Bozorth, NFIQ2, SIVV). You cannot get true CPU-parallel native extract in one Python/Rust process; extra threads queue on this lock.
- **True parallel extract**: run **multiple OS processes** (e.g. Gunicorn/Uvicorn workers, `multiprocessing.Pool`, Kubernetes replicas). Each process loads its own `libnbis.so` and mutex; throughput scales with worker count.
- **Defense in depth**: Python embedders may still use a module-level `RLock` around all `nbis` calls; it should not be required if you use `nbis-python` ≥ 0.1.12 with the native mutex.
- **NFIQ2**: one C++ model per `Nfiq2`; `NbisExtractor` also uses `Mutex<Nfiq2>` per instance (nested under the native mutex during extract).
- **`Nfiq2` is not `Clone`** (avoids double `destroy`).
- **Pure Rust** (`load_iso_19794_2_2011`, `to_iso_19794_2_2011`, `compare_iso_19794_2_2011`, and legacy `load_iso_19794_2_2005`): template load/encode need no native lock; `compare_iso_19794_2_2011` calls Bozorth via `Minutiae::compare` and uses the native mutex.

## Bozorth match / 1:N search

- Bozorth C uses process-global tables; never call match concurrently without the native mutex (already held by `Minutiae::compare`).
- **qq[] overflow paths** historically called `fprintf(errorfp, …)` with `errorfp == NULL` and NULL probe/gallery filenames. That is undefined behavior and segfaults under gallery search when overflow is hit. Fixed in 0.1.17+:
  - `errorfp` is initialized to a silent `/dev/null` (or `stderr` fallback), never left NULL
  - filename getters never return NULL
  - overflow logs use `BZ_FPRINTF` (NULL-safe)
- On overflow, Rust still maps score `4000` → `0` (no match).

## Input validation

- Grayscale dimensions validated before SIVV / morph center FFI.
- Minutiae count from C capped and checked (`num`, `alloc`, non-null `list`).
- ISO template load: header, length, minutiae count, and per-record bounds checked.
- ISO encode: coordinates must fit `u16`; returns `NbisError::CoordinateOutOfRange`.

## C++ wrapper hardening

- `nfiq_wrapper.cpp`: `malloc` failures and C++ exceptions call `nfiq2wrapper_free_results` before returning.
- `sivv_wrapper.cpp`: clones image data (no in-place OpenCV writes on Rust slice); null checks on inputs/outputs.

## Remaining embedder responsibilities

- Valid fingerprint image bytes (corrupt WSQ/PNG may still crash inside legacy NBIS parsers).
- Match OpenCV **4.13** at runtime to the wheel build.
- Rebuild/publish a new wheel after changing native dependencies.
