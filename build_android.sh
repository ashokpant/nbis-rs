#!/bin/bash
set -euo pipefail

echo "Checking prerequisites..."

if ! command -v cargo &> /dev/null; then
  echo "Error: Cargo is required but not installed."
  exit 1
fi

if [ -z "${ANDROID_NDK_ROOT:-}" ]; then
  echo "Error: ANDROID_NDK_ROOT environment variable not set"
  exit 1
fi

if [ ! -d "$ANDROID_NDK_ROOT" ]; then
  echo "Error: ANDROID_NDK_ROOT directory does not exist"
  exit 1
fi

echo "Checking for cargo-ndk..."
if ! command -v cargo-ndk &> /dev/null; then
  echo "Installing cargo-ndk..."
  cargo install cargo-ndk
fi

echo "Setting up Rust targets..."
rustup target add aarch64-linux-android armv7-linux-androideabi

TARGETS=("aarch64-linux-android" "armv7-linux-androideabi")

for TARGET in "${TARGETS[@]}"; do
  echo "Building for $TARGET..."
  cargo ndk -t "$TARGET" -o ./target/android build --release
done

echo "Android build completed successfully"
ls -lh ./target/android/*/release/*.so 2>/dev/null || true

# Clear jniLibs output
echo "🧹 Cleaning existing jniLibs..."
rm -f "$JNILIBS_DIR/arm64-v8a/"*.so
rm -f "$JNILIBS_DIR/armeabi-v7a/"*.so

# Copy .so files into correct ABI folders
for TARGET in "${TARGETS[@]}"; do
  case "$TARGET" in
    "aarch64-linux-android")
      ABI="arm64-v8a"
      ;;
    "armv7-linux-androideabi")
      ABI="armeabi-v7a"
      ;;
    *)
      echo "❌ Unknown target: $TARGET"
      exit 1
      ;;
  esac

  echo "📁 Copying .so for $ABI..."
  mkdir -p "$JNILIBS_DIR/$ABI"

  # Adjust the library name as needed
  LIB_NAME="libnbis.so"
  cp "target/android/$ABI/$LIB_NAME" "$JNILIBS_DIR/$ABI/"
done

HOST_OS=$(uname -s)
if [ "$HOST_OS" = "Darwin" ]; then
  PREBUILT="darwin-x86_64"
else
  PREBUILT="linux-x86_64"
fi

cp $ANDROID_NDK_HOME/toolchains/llvm/prebuilt/$PREBUILT/sysroot/usr/lib/arm-linux-androideabi/libc++_shared.so bindings/android/app/src/main/jniLibs/armeabi-v7a/
cp $ANDROID_NDK_HOME/toolchains/llvm/prebuilt/$PREBUILT/sysroot/usr/lib/aarch64-linux-android/libc++_shared.so bindings/android/app/src/main/jniLibs/arm64-v8a/

echo "✅ Done. .so files copied to $JNILIBS_DIR"

# Detect platform-specific dynamic library extension
case "$(uname)" in
  Darwin)   LIB_EXT="dylib" ;;
  Linux)    LIB_EXT="so" ;;
  *)        echo "❌ Unsupported platform: $(uname)" && exit 1 ;;
esac

cargo build --release
cargo run --bin uniffi-bindgen generate \
          --library "target/release/libnbis.${LIB_EXT}" \
          --language kotlin \
          --out-dir bindings/android/app/src/main/java/ai/seventhsense/sdk/nbis

# Move to sensible folder without uniffi opinionation
mv bindings/android/app/src/main/java/ai/seventhsense/sdk/nbis/uniffi/nbis/nbis.kt \
    bindings/android/app/src/main/java/ai/seventhsense/sdk/nbis.kt
rm -rf bindings/android/app/src/main/java/ai/seventhsense/sdk/nbis


# Root directory of generated Kotlin files
TARGET_DIR="bindings/android/app/src/main/java/ai/seventhsense/sdk"

# Old and new package names
OLD_PACKAGE="package uniffi.nbis"
NEW_PACKAGE="package ai.seventhsense.sdk.nbis"

# Recursively find all .kt files and replace the package line
echo "🔍 Replacing package '$OLD_PACKAGE' with '$NEW_PACKAGE' in $TARGET_DIR"

# Use sed with platform detection
if [[ "$OSTYPE" == "darwin"* ]]; then
  # macOS sed requires an empty string after -i
  find "$TARGET_DIR" -name "*.kt" -exec sed -i '' "s|^$OLD_PACKAGE|$NEW_PACKAGE|" {} +
else
  # Linux sed
  find "$TARGET_DIR" -name "*.kt" -exec sed -i "s|^$OLD_PACKAGE|$NEW_PACKAGE|" {} +
fi

# Build the aar file
echo "📦 Building Android AAR..."
cd bindings/android
./gradlew :app:assembleRelease
if [ $? -ne 0 ]; then
  echo "❌ Failed to build Android AAR"
  exit 1
fi
# Rename the AAR file
AAR_FILE=$(find app/build/outputs/aar -name "*.aar" | head -n 1)
if [ -z "$AAR_FILE" ]; then
  echo "❌ No AAR file found"
  exit 1
fi

mkdir -p ../../dist
mv "$AAR_FILE" "../../dist/nbis-sdk.aar"
