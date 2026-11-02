//! Quota, leases, context/source validation, and eviction through the cache interface.

use std::sync::{Arc, Barrier};
use vizzi::syntax_cache::{SyntaxCache, SyntaxKey};
use vizzi_core::{
    analysis::{Languages, SyntaxState},
    model::{ContentId, WorkspaceId},
};

fn state() -> SyntaxState {
    Languages::builtin()
        .unwrap()
        .select("rust")
        .unwrap()
        .create()
        .unwrap()
        .analyze_with_state(b"fn example() {}", None, usize::MAX, &|| false)
        .unwrap()
        .unwrap()
        .state
        .unwrap()
}
fn key(path: &str, state: &SyntaxState) -> SyntaxKey {
    SyntaxKey::new(
        &WorkspaceId::derive(&[b"cache fixture"]),
        path,
        state.context(),
    )
}

#[test]
fn active_leases_charge_old_and_new_state_and_invalidation_cannot_reinsert_an_entry() {
    let a = state();
    let weight = a.weight();
    let akey = key("a.rs", &a);
    let bkey = key("b.rs", &a);
    let content = a.content().clone();
    let cache = SyntaxCache::new(weight * 3, weight, 2).unwrap();
    assert!(cache.lease(akey.clone(), false).is_none());
    assert!(
        cache
            .lease(akey.clone(), true)
            .unwrap()
            .finish(Some(a), &content)
    );
    assert!(
        cache
            .lease(bkey.clone(), true)
            .unwrap()
            .finish(Some(state()), &content)
    );
    let mut a = cache.lease(akey.clone(), false).unwrap();
    assert_eq!(cache.stats().charged_weight, weight * 3);
    assert_eq!(cache.stats().active_leases, 1);
    assert!(cache.lease(akey.clone(), true).is_none());
    assert!(cache.lease(bkey.clone(), true).is_none());
    assert_eq!(cache.stats().bypassed, 2);
    cache.invalidate(&WorkspaceId::derive(&[b"cache fixture"]), "a.rs");
    let previous = a.take_previous();
    assert!(!a.finish(previous, &content));
    assert_eq!(cache.stats().charged_weight, weight);
    assert!(cache.lease(akey, false).is_none());
    drop(cache.lease(bkey, false).unwrap());
    assert_eq!(cache.stats().charged_weight, 0);
    assert_eq!(cache.stats().active_leases, 0);
}

#[test]
fn least_recent_idle_entries_are_evicted_and_wrong_content_is_never_admitted() {
    let first = state();
    let weight = first.weight();
    let content = first.content().clone();
    let cache = SyntaxCache::new(weight * 4, weight, 2).unwrap();
    for path in ["a.rs", "b.rs", "c.rs"] {
        let value = state();
        assert!(
            cache
                .lease(key(path, &value), true)
                .unwrap()
                .finish(Some(value), &content)
        );
    }
    assert!(cache.lease(key("a.rs", &first), false).is_none());
    assert_eq!(cache.stats().evicted, 1);
    let lease = cache.lease(key("b.rs", &first), false).unwrap();
    assert!(!lease.finish(Some(state()), &ContentId::derive(&[b"different source"])));
    assert_eq!(cache.stats().idle_entries, 1);
    assert_eq!(cache.stats().charged_weight, weight);
    assert_eq!(cache.stats().active_leases, 0);
    for (budget, entry, count) in [(0, 1, 1), (1, 0, 1), (1, 2, 1), (1, 1, 0)] {
        assert!(SyntaxCache::new(budget, entry, count).is_err());
    }
}

#[test]
fn context_workspace_and_returned_state_limits_are_independent_of_a_path_match() {
    let value = state();
    let weight = value.weight();
    let content = value.content().clone();
    let cache = SyntaxCache::new(weight * 3, weight, 3).unwrap();
    let original_key = key("a.rs", &value);
    assert!(
        cache
            .lease(original_key.clone(), true)
            .unwrap()
            .finish(Some(value), &content)
    );
    let mut descriptor = state().context().clone();
    descriptor.description.push_str("changed");
    assert!(
        cache
            .lease(
                SyntaxKey::new(
                    &WorkspaceId::derive(&[b"cache fixture"]),
                    "a.rs",
                    &descriptor
                ),
                false
            )
            .is_none()
    );
    assert!(
        cache
            .lease(
                SyntaxKey::new(
                    &WorkspaceId::derive(&[b"other workspace"]),
                    "a.rs",
                    state().context()
                ),
                false
            )
            .is_none()
    );
    let languages = Languages::builtin().unwrap();
    let python = languages
        .select("python")
        .unwrap()
        .create()
        .unwrap()
        .analyze_with_state(b"", None, usize::MAX, &|| false)
        .unwrap()
        .unwrap()
        .state
        .unwrap();
    let expected = python.content().clone();
    assert!(
        !cache
            .lease(original_key.clone(), false)
            .unwrap()
            .finish(Some(python), &expected)
    );
    let large = languages
        .select("rust")
        .unwrap()
        .create()
        .unwrap()
        .analyze_with_state(b"fn a() {} fn b() {} fn c() {}", None, usize::MAX, &|| {
            false
        })
        .unwrap()
        .unwrap()
        .state
        .unwrap();
    let expected = large.content().clone();
    assert!(large.weight() > weight);
    assert!(
        !cache
            .lease(original_key, true)
            .unwrap()
            .finish(Some(large), &expected)
    );
    assert_eq!(cache.stats().charged_weight, 0);
    for field in ["id", "language", "description"] {
        let value = state();
        let content = value.content().clone();
        let mut descriptor = value.context().clone();
        match field {
            "id" => descriptor.id = vizzi_core::model::ContextId::derive(&[b"changed id"]),
            "language" => descriptor.language = "different language".into(),
            _ => descriptor.description = "different description".into(),
        }
        let requested = SyntaxKey::new(
            &WorkspaceId::derive(&[b"cache fixture"]),
            "a.rs",
            &descriptor,
        );
        assert!(
            !cache
                .lease(requested, true)
                .unwrap()
                .finish(Some(value), &content)
        );
    }
}

#[test]
fn shared_concurrent_leases_never_exceed_admission_weight_or_entry_limit() {
    let value = state();
    let weight = value.weight();
    let cache = Arc::new(SyntaxCache::new(weight * 4, weight, 3).unwrap());
    let barrier = Arc::new(Barrier::new(8));
    std::thread::scope(|scope| {
        for index in 0..8 {
            let cache = cache.clone();
            let barrier = barrier.clone();
            scope.spawn(move || {
                barrier.wait();
                for _ in 0..20 {
                    let value = state();
                    if let Some(lease) = cache.lease(key(&format!("{index}.rs"), &value), true) {
                        let content = value.content().clone();
                        let stats = cache.stats();
                        assert!(stats.charged_weight <= weight * 4);
                        assert!(stats.idle_entries + stats.active_leases <= 3);
                        lease.finish(Some(value), &content);
                    }
                }
            });
        }
    });
    assert_eq!(cache.stats().active_leases, 0);
    assert!(cache.stats().peak_charged_weight <= weight * 4);
    assert!(cache.stats().idle_entries <= 3);
}

#[test]
fn weight_pressure_evicts_idle_entries_before_the_entry_count_limit() {
    let value = state();
    let weight = value.weight();
    let content = value.content().clone();
    let cache = SyntaxCache::new(weight * 2, weight, 8).unwrap();
    let first = cache.lease(key("a.rs", &value), true).unwrap();
    let second = cache.lease(key("b.rs", &value), true).unwrap();
    assert_eq!(cache.stats().charged_weight, weight * 2);
    first.finish(Some(state()), &content);
    second.finish(Some(state()), &content);
    let third = cache.lease(key("c.rs", &value), true).unwrap();
    assert_eq!(cache.stats().charged_weight, weight * 2);
    assert_eq!(cache.stats().evicted, 1);
    third.finish(Some(state()), &content);
    cache.invalidate(&WorkspaceId::derive(&[b"cache fixture"]), "b.rs");
    assert_eq!(cache.stats().charged_weight, weight);
    cache.invalidate(&WorkspaceId::derive(&[b"cache fixture"]), "c.rs");
    assert_eq!(cache.stats().charged_weight, 0);
    let policy = SyntaxCache::for_budget(536_870_912).unwrap().stats();
    assert_eq!(policy.entry_weight_limit, 268_435_456);
    assert_eq!(policy.budget_weight, 536_870_912);
}

#[test]
fn inspecting_an_entry_preserves_lease_ownership_and_lru_order() {
    use vizzi::syntax_cache::SyntaxEntry;
    let value = state();
    let weight = value.weight();
    let content = value.content().clone();
    let cache = SyntaxCache::new(weight * 4, weight, 2).unwrap();
    let first = key("a.rs", &value);
    let second = key("b.rs", &value);
    assert_eq!(cache.entry(&first), None);
    for requested in [&first, &second] {
        let lease = cache.lease(requested.clone(), true).unwrap();
        assert_eq!(cache.entry(requested), Some(SyntaxEntry::Leased));
        lease.finish(Some(state()), &content);
    }
    assert_eq!(
        cache.entry(&first),
        Some(SyntaxEntry::Idle {
            weight,
            source_bytes: value.source_bytes(),
            nodes: value.nodes(),
        })
    );
    let before = cache.stats();
    assert!(matches!(
        cache.entry(&first),
        Some(SyntaxEntry::Idle { .. })
    ));
    assert_eq!(cache.stats(), before);
    let third = cache.lease(key("c.rs", &value), true).unwrap();
    assert_eq!(
        cache.entry(&first),
        None,
        "inspection must not refresh LRU order"
    );
    assert!(matches!(
        cache.entry(&second),
        Some(SyntaxEntry::Idle { .. })
    ));
    drop(third);
    let lease = cache.lease(second.clone(), false).unwrap();
    cache.invalidate(&WorkspaceId::derive(&[b"cache fixture"]), "b.rs");
    assert_eq!(cache.entry(&second), Some(SyntaxEntry::Leased));
    drop(lease);
    assert_eq!(cache.entry(&second), None);
}

#[test]
fn maximum_new_state_reservations_can_evict_a_large_idle_tree_for_a_small_edit() {
    let source: String = (0..100).map(|i| format!("fn item_{i}() {{}}\n")).collect();
    let large = Languages::builtin()
        .unwrap()
        .select("rust")
        .unwrap()
        .create()
        .unwrap()
        .analyze_with_state(source.as_bytes(), None, usize::MAX, &|| false)
        .unwrap()
        .unwrap()
        .state
        .unwrap();
    let small = state();
    let weight = large.weight();
    assert!(weight > 2 * small.weight());
    let large_key = key("large.rs", &large);
    let small_key = key("ordinary.rs", &small);
    let cache = SyntaxCache::new(2 * weight, weight, 8).unwrap();
    let content = large.content().clone();
    cache
        .lease(large_key.clone(), true)
        .unwrap()
        .finish(Some(large), &content);
    let content = small.content().clone();
    cache
        .lease(key("other.rs", &small), true)
        .unwrap()
        .finish(Some(small), &content);
    assert!(cache.entry(&large_key).is_some());
    let before = cache.stats();
    assert_eq!(before.idle_entries, 2);
    assert_eq!(before.evicted, 0);
    let lease = cache.lease(small_key, true).unwrap();
    assert_eq!(lease.allowance(), weight);
    lease.finish(Some(state()), &content);
    assert_eq!(cache.entry(&large_key), None);
    assert_eq!(cache.stats().evicted, 1);
    assert_eq!(cache.stats().idle_entries, 2);
    assert!(cache.stats().peak_charged_weight <= 2 * weight);
}
