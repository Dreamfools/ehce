pub type OrderMapEntry<'a, K, V> = ordermap::map::Entry<'a, K, V>;
pub type DashMapEntry<'a, K, V> = dashmap::Entry<'a, K, V>;

#[allow(clippy::disallowed_types)]
pub type Hasher = bevy::platform::hash::DefaultHasher<'static>;
pub type BuildHasher = bevy::platform::hash::FixedHasher;

// DOS is of no concern to us
pub type OrderMap<K, V> = ordermap::OrderMap<K, V, BuildHasher>;
pub type OrderSet<V> = ordermap::OrderSet<V, BuildHasher>;

#[allow(clippy::disallowed_types)]
pub type DashMap<K, V> = dashmap::DashMap<K, V, BuildHasher>;
#[allow(clippy::disallowed_types)]
pub type DashSet<V> = dashmap::DashSet<V, BuildHasher>;

pub type DashMapRef<'a, K, V> = dashmap::mapref::one::Ref<'a, K, V>;

#[allow(clippy::disallowed_types)]
pub type HashMap<K, V> = std::collections::HashMap<K, V, BuildHasher>;
#[allow(clippy::disallowed_types)]
pub type HashSet<V> = std::collections::HashSet<V, BuildHasher>;

pub fn hash_of<T: std::hash::Hash>(t: &T) -> u64 {
    let mut h = std::hash::BuildHasher::build_hasher(&BuildHasher::default());
    t.hash(&mut h);
    std::hash::Hasher::finish(&h)
}