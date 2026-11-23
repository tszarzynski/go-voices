//! Which patches to list: the place in the tree and the search.

use alloc::{
    collections::{BTreeMap, BTreeSet},
    string::String,
    vec::Vec,
};

use catalogue::{Bank, Catalogue, Category, Query};

use crate::app::Place;

/// What the list is narrowed to.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Filters {
    pub place: Place,
    pub query: String,
}

impl Filters {
    /// Something is typed in the search field.
    pub fn searching(&self) -> bool {
        !self.query.trim().is_empty()
    }
}

/// The patches a place starts from: the favourites (catalogue order), the
/// recents (most recent first) or the whole catalogue.
fn place_ids(
    catalogue: &Catalogue,
    place: Place,
    favourites: &BTreeSet<usize>,
    recents: &[usize],
) -> Vec<usize> {
    match place {
        Place::Favourites => favourites.iter().copied().collect(),
        Place::Recent => recents.to_vec(),
        Place::Presets => Vec::new(),
        Place::All | Place::Bank(_) | Place::Category { .. } => {
            (0..catalogue.patches().len()).collect()
        }
    }
}

/// The ids to list, best match first. Ties keep the place's order.
pub fn results(
    catalogue: &Catalogue,
    filters: &Filters,
    favourites: &BTreeSet<usize>,
    recents: &[usize],
) -> Vec<usize> {
    let place = filters.place;
    let query = Query::new(&filters.query);
    let mut ranked: Vec<(u8, usize)> = place_ids(catalogue, place, favourites, recents)
        .into_iter()
        .filter_map(|id| {
            let patch = catalogue.get(id)?;
            let wanted = place.bank().is_none_or(|bank| patch.bank() == bank)
                && place.category().is_none_or(|c| patch.category == c);
            wanted
                .then(|| query.rank(patch))
                .flatten()
                .map(|rank| (rank, id))
        })
        .collect();
    // Stable: equal ranks keep the place's order.
    ranked.sort_by_key(|&(rank, _)| rank);
    ranked.into_iter().map(|(_, id)| id).collect()
}

/// A bank in the tree: the bank, its patch count, and its categories with
/// theirs.
pub type BankTree = (Bank, usize, Vec<(Category, usize)>);

/// The tree under the banks, in the keyboard's order.
pub fn tree(catalogue: &Catalogue) -> Vec<BankTree> {
    Bank::ALL
        .into_iter()
        .map(|bank| {
            let mut categories: BTreeMap<Category, usize> = BTreeMap::new();
            for patch in catalogue.patches().iter().filter(|p| p.bank() == bank) {
                *categories.entry(patch.category).or_default() += 1;
            }
            let total = categories.values().sum();
            (bank, total, categories.into_iter().collect())
        })
        .collect()
}

/// The item one place from `current` in `ids`, wrapping around. Without a
/// current one in the list, the first (forward) or last (back).
pub fn step<T: PartialEq + Copy>(ids: &[T], current: Option<T>, forward: bool) -> Option<T> {
    let last = ids.len().checked_sub(1)?;
    let position = current.and_then(|id| ids.iter().position(|&i| i == id));
    let next = match (position, forward) {
        (None, true) => 0,
        (None, false) => last,
        (Some(p), true) => {
            if p == last {
                0
            } else {
                p + 1
            }
        }
        (Some(p), false) => p.checked_sub(1).unwrap_or(last),
    };
    ids.get(next).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalogue() -> Catalogue {
        Catalogue::parse(
            "Warm Pad\tSPD\t87\t71\t1\n\
             Grand Piano\tPNO\t87\t64\t1\n\
             Soft Warm Pad\tSPD\t87\t71\t2\n\
             Piano 1\tPNO\t121\t0\t1\n",
        )
        .unwrap()
    }

    fn filters(place: Place, query: &str) -> Filters {
        Filters {
            place,
            query: query.into(),
        }
    }

    #[test]
    fn searches_and_ranks() {
        let ids = results(
            &catalogue(),
            &filters(Place::All, "pad"),
            &BTreeSet::new(),
            &[],
        );
        assert_eq!(ids, [0, 2]);
        let ids = results(
            &catalogue(),
            &filters(Place::All, "piano"),
            &BTreeSet::new(),
            &[],
        );
        assert_eq!(ids, [3, 1], "name prefix first");
    }

    #[test]
    fn places_narrow_to_a_bank_or_a_category() {
        let none = BTreeSet::new();
        let f = filters(Place::Bank(Bank::Gm2), "");
        assert_eq!(results(&catalogue(), &f, &none, &[]), [3]);
        let f = filters(
            Place::Category {
                bank: Bank::JunoDs,
                category: Category::SoftPad,
            },
            "",
        );
        assert_eq!(results(&catalogue(), &f, &none, &[]), [0, 2]);
    }

    #[test]
    fn the_tree_counts_each_bank_and_category() {
        let tree = tree(&catalogue());
        assert_eq!(
            tree[0],
            (
                Bank::JunoDs,
                3,
                Vec::from([(Category::Piano, 1), (Category::SoftPad, 2)])
            )
        );
        assert_eq!(tree[1], (Bank::Gm2, 1, Vec::from([(Category::Piano, 1)])));
        assert_eq!(tree[2], (Bank::Gm2Drums, 0, Vec::new()));
    }

    #[test]
    fn favourites_and_recents_are_places() {
        let favourites = BTreeSet::from([2, 0]);
        let ids = results(
            &catalogue(),
            &filters(Place::Favourites, ""),
            &favourites,
            &[],
        );
        assert_eq!(ids, [0, 2]);
        let ids = results(
            &catalogue(),
            &filters(Place::Recent, ""),
            &favourites,
            &[3, 1],
        );
        assert_eq!(ids, [3, 1], "most recent first");
    }

    #[test]
    fn steps_wrap_around() {
        let ids = [4, 7, 9];
        assert_eq!(step(&ids, Some(7), true), Some(9));
        assert_eq!(step(&ids, Some(9), true), Some(4));
        assert_eq!(step(&ids, Some(4), false), Some(9));
        assert_eq!(step(&ids, None, true), Some(4));
        assert_eq!(step(&ids, Some(1), false), Some(9));
        assert_eq!(step::<usize>(&[], None, true), None);
    }
}
