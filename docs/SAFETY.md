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

## Threading (0.1.18+)

| Path | In-process parallelism | Mechanism |
|------|------------------------|-----------|
| ISO load / encode | Yes | Pure Rust, unlocked |
| Image decode (before extract) | Yes | Pure Rust, outside extract lock |
| **Bozorth match** (`Minutiae::compare`, `compare_iso_*`) | **Yes** | Thread-local Bozorth C workspace |
| mindtct extract / SIVV / NFIQ2 | No (serialized) | `EXTRACT_LOCK` only |
| Multi-process extract | Yes (recommended) | Process pool / workers |

- **Parallel match**: Bozorth globals (`colp`, `qq`, scratch in `bozorth3.c`, sort stack, etc.) are `BZ_THREAD_LOCAL`. Concurrent compares in one process are safe and produce the same scores as serial.
- **Serialized extract**: `extract_minutiae` holds `EXTRACT_LOCK` only around mindtct/SIVV/NFIQ2 (image decode is outside the lock). True parallel extract still needs **multiple OS processes**.
- **Batch 1:N**: `NbisExtractor::compare_iso_19794_2_2011_batch` runs Bozorth on a Rayon pool sized by `NBIS_BOZORTH_THREADS` (default `min(n_cpus, 8)`). Set `NBIS_BOZORTH_THREADS=1` to force sequential.
- **Defense in depth**: Python embedders may keep an `RLock` around **extract** paths; match/batch no longer need a process-wide Python lock when using `nbis-python` ≥ 0.1.18.
- **NFIQ2**: one C++ model per `Nfiq2`; `NbisExtractor` also uses `Mutex<Nfiq2>` per instance (nested under the extract lock).
- **`Nfiq2` is not `Clone`** (avoids double `destroy`).

## Bozorth match / 1:N search

- Concurrent Bozorth in one process is supported (≥ 0.1.18). Prefer `compare_iso_19794_2_2011_batch` for large galleries.
- **qq[] overflow paths** historically called `fprintf(errorfp, …)` with `errorfp == NULL` and NULL probe/gallery filenames. Fixed in 0.1.17+:
  - `errorfp` is initialized to a silent `/dev/null` (or `stderr` fallback), never left NULL; TLS threads call `nbis_bozorth_ensure_errorfp()` from `bozorth_main`
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
- For extract throughput and SIGSEGV isolation on Linux, prefer a **process** pool over in-process extract threads.
