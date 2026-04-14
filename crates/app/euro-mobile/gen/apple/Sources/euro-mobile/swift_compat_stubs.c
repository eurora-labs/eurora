// Stub definitions for Swift compatibility symbols that are missing in Xcode 26.
//
// Xcode 26 (Swift 6.2) no longer ships the swiftCompatibility56 and
// swiftCompatibilityPacks back-deployment libraries. However, the Swift
// objects inside libapp.a (compiled by swift-rs via Swift Package Manager)
// still reference these symbols because SPM builds against the host (macOS)
// platform regardless of the -target flag passed to swiftc.
//
// Providing empty definitions here satisfies the linker without any runtime
// effect – the compatibility thunks are never actually called on iOS 16+.

#include <stdint.h>

// swiftCompatibility56
uint8_t _swift_FORCE_LOAD_$_swiftCompatibility56 = 0;

// swiftCompatibilityPacks
uint8_t _swift_FORCE_LOAD_$_swiftCompatibilityPacks = 0;