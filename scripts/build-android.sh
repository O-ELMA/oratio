#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

PROFILE="${1:-release}"
ABI="${2:-aarch64-linux-android}"

case "$PROFILE" in
  debug|release) ;;
  *) echo "usage: $0 [debug|release] [aarch64-linux-android|x86_64-linux-android]" >&2; exit 2 ;;
esac

if [[ "$PROFILE" == "release" ]]; then
  BUILD_DIR="release"
  DX_FLAGS=(--release)
else
  BUILD_DIR="debug"
  DX_FLAGS=()
fi

if [[ -n "${ANDROID_HOME:-}" ]]; then
  export ANDROID_SDK_ROOT="${ANDROID_SDK_ROOT:-$ANDROID_HOME}"
fi

ANDROID_NDK="${ANDROID_NDK_HOME:-${ANDROID_NDK_ROOT:-}}"
if [[ -z "$ANDROID_NDK" ]]; then
  echo "ANDROID_NDK_HOME or ANDROID_NDK_ROOT must point at the Android NDK" >&2
  exit 1
fi

case "$ABI" in
  aarch64-linux-android) TRIPLE="aarch64-linux-android26"; LIB_TRIPLE="aarch64-linux-android"; CMAKE_ABI="arm64-v8a"; JNI_ABI="arm64-v8a"; ARCH_FLAGS="-march=armv8-a" ;;
  x86_64-linux-android) TRIPLE="x86_64-linux-android26"; LIB_TRIPLE="x86_64-linux-android"; CMAKE_ABI="x86_64"; JNI_ABI="x86_64"; ARCH_FLAGS="" ;;
  *) echo "unsupported ABI: $ABI" >&2; exit 2 ;;
esac

NDK_BIN="$ANDROID_NDK/toolchains/llvm/prebuilt/linux-x86_64/bin"
TARGET_ENV="${ABI//-/_}"
TARGET_UPPER="${ABI^^}"
TARGET_UPPER="${TARGET_UPPER//-/_}"

export CMAKE_TOOLCHAIN_FILE="$ANDROID_NDK/build/cmake/android.toolchain.cmake"
export CMAKE_SYSTEM_NAME=Android
export CMAKE_SYSTEM_VERSION=26
export CMAKE_ANDROID_ARCH_ABI="$CMAKE_ABI"
export CMAKE_ANDROID_NDK="$ANDROID_NDK"
export CMAKE_ANDROID_STL_TYPE=c++_shared
export "CC_${TARGET_ENV}=$NDK_BIN/${TRIPLE}-clang"
export "CXX_${TARGET_ENV}=$NDK_BIN/${TRIPLE}-clang++"
export "AR_${TARGET_ENV}=$NDK_BIN/llvm-ar"
export "CFLAGS_${TARGET_ENV}=--target=${TRIPLE} ${ARCH_FLAGS}"
export "CXXFLAGS_${TARGET_ENV}=--target=${TRIPLE} ${ARCH_FLAGS}"
export "CARGO_TARGET_${TARGET_UPPER}_LINKER=$NDK_BIN/${TRIPLE}-clang"
export WHISPER_DONT_GENERATE_BINDINGS=1

echo "==> dx build ($PROFILE, $ABI)"
dx build --platform android --target "$ABI" "${DX_FLAGS[@]}"

APPDIR="target/dx/oratio/$BUILD_DIR/android/app"
if [[ ! -d "$APPDIR" ]]; then
  echo "build output not found: $APPDIR" >&2
  exit 1
fi

echo "==> installing Oratio icon resources"
cp -r android/res/. "$APPDIR/app/src/main/res/"

echo "==> installing libc++_shared.so"
mkdir -p "$APPDIR/app/src/main/jniLibs/$JNI_ABI"
cp "$ANDROID_NDK/toolchains/llvm/prebuilt/linux-x86_64/sysroot/usr/lib/$LIB_TRIPLE/libc++_shared.so" "$APPDIR/app/src/main/jniLibs/$JNI_ABI/"

echo "==> repackaging APK"
(
  cd "$APPDIR"
  ./gradlew assembleDebug --quiet --no-daemon
)

mkdir -p dist
ARCH="${ABI%-linux-android}"
OUT="dist/oratio-${ARCH}-${PROFILE}.apk"
cp "$APPDIR/app/build/outputs/apk/debug/app-debug.apk" "$OUT"

echo
echo "APK:  $OUT"
echo "Size: $(du -h "$OUT" | cut -f1)"
sha256sum "$OUT"
