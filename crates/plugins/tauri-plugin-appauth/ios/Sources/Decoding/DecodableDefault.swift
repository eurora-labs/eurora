// SPDX-License-Identifier: Apache-2.0

import Foundation

/// Supplies the default value used by `DecodableDefault` when a JSON field is
/// absent or `null`. Conformers are zero-state enums whose only purpose is to
/// vend a static default of their associated `Value` type.
protocol DecodableDefaultSource {
    associatedtype Value: Decodable
    static var defaultValue: Value { get }
}

/// Property wrapper that makes a `Decodable` field tolerate a missing or
/// `null` JSON value by falling back to a compile-time default declared on the
/// property itself. Eliminates per-struct `init(from:)` boilerplate and keeps
/// defaults from drifting out of sync between platforms.
///
/// Pair with the typealiases below — `@DefaultTrue`, `@DefaultEmptyArray<T>`,
/// `@DefaultEmptyDictionary<K, V>` — rather than spelling out the source enum.
@propertyWrapper
struct DecodableDefault<Source: DecodableDefaultSource>: Decodable {
    typealias Value = Source.Value

    var wrappedValue: Value

    init() {
        wrappedValue = Source.defaultValue
    }

    init(from decoder: Decoder) throws {
        wrappedValue = try Value(from: decoder)
    }
}

extension KeyedDecodingContainer {
    /// Lets the synthesized `init(from:)` of any struct using `DecodableDefault`
    /// fall back to the wrapper's default when the key is absent or explicitly
    /// `null`. Picked over the stdlib's generic `decode<T: Decodable>` because
    /// `DecodableDefault<Source>` is a more specific match.
    func decode<Source>(
        _ type: DecodableDefault<Source>.Type,
        forKey key: Key
    ) throws -> DecodableDefault<Source> {
        try decodeIfPresent(type, forKey: key) ?? DecodableDefault<Source>()
    }
}

// MARK: - Concrete defaults

enum DecodableDefaultSources {
    enum True: DecodableDefaultSource {
        static var defaultValue: Bool { true }
    }

    enum EmptyArray<Element: Decodable>: DecodableDefaultSource {
        static var defaultValue: [Element] { [] }
    }

    enum EmptyDictionary<Key: Decodable & Hashable, Value: Decodable>: DecodableDefaultSource {
        static var defaultValue: [Key: Value] { [:] }
    }
}

typealias DefaultTrue = DecodableDefault<DecodableDefaultSources.True>
typealias DefaultEmptyArray<E: Decodable> = DecodableDefault<DecodableDefaultSources.EmptyArray<E>>
typealias DefaultEmptyDictionary<K: Decodable & Hashable, V: Decodable> =
    DecodableDefault<DecodableDefaultSources.EmptyDictionary<K, V>>
