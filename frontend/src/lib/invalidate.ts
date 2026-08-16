/// What the browser has to forget after a write.

import { cache, CachePolicy } from '$houdini';

/// Everything, rather than the type that was written.
///
/// Naming a type (`markStale('Expense')`) only reaches queries that hold records of it, and
/// the queries an expense write actually invalidates hold none: the monthly totals, the
/// computed account balances and the usage counts the pickers sort on are all aggregates the
/// server derives from rows the document never selected. Nothing marks those stale, so the
/// dashboard and the analysis tabs went on showing the numbers from before the write until
/// something else happened to refetch them.
///
/// There is no cheap way to name what a write touches -- one expense moves a category total,
/// two account balances and both usage counts -- and a write here is somebody pressing a
/// button, not something in a loop. So the whole cache goes.
export const invalidate = () => cache.markStale();

/// The policy a refetch that follows a write has to use.
///
/// `invalidate()` above is what makes *other* views refetch when they are next looked at.
/// This is for the view doing the writing, which reloads immediately: the default
/// `CacheOrNetwork` would go to the network here too, but only because the cache was marked
/// stale a moment earlier, and "the list I just edited is correct" is too load-bearing to
/// rest on that bookkeeping.
export const AFTER_WRITE = { policy: CachePolicy.NetworkOnly };
