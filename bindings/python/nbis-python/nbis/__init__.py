"""Python bindings for NBIS fingerprint processing (Rust + UniFFI)."""

from .nbis import (
    Minutia,
    MinutiaKind,
    Minutiae,
    NbisError,
    NbisExtractor,
    NbisExtractorSettings,
    Nfiq2Result,
    Nfiq2Value,
    Position,
    Point,
    Roi,
    new_nbis_extractor,
)

# Backward-compatible alias (Rust type is `ROI`, UniFFI exposes `Roi` in Python).
ROI = Roi

__version__ = "0.1.5"
__all__ = [
    "Minutia",
    "MinutiaKind",
    "Minutiae",
    "NbisError",
    "NbisExtractor",
    "NbisExtractorSettings",
    "Nfiq2Result",
    "Nfiq2Value",
    "Position",
    "Point",
    "Roi",
    "ROI",
    "new_nbis_extractor",
]
