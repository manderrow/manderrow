//! The `ModIndex` QML singleton, which fetches and searches the games' mod indexes.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qlist.h");
        type QList_i32 = cxx_qt_lib::QList<i32>;
        type QList_bool = cxx_qt_lib::QList<bool>;
        include!("manderrow/src/qt/call.cxxqt.h");
        type Call = crate::qt::call::Call;
    }

    /// What mods can be sorted by.
    #[qenum(ModIndex)]
    enum ModSortColumn {
        /// How well they match the query.
        Relevance,
        /// The downloads of all of their versions.
        Downloads,
        Name,
        Owner,
        /// The size of their latest version.
        Size,
    }

    #[auto_cxx_name]
    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        type ModIndex = super::ModIndexRust;

        /// Fetches a game's mod index, unless it has been already. With `refresh`, it's fetched
        /// again. The call's task is the fetch, if there is one.
        #[qinvokable]
        fn fetch(self: &ModIndex, game: &QString, refresh: bool) -> *mut Call;

        /// Searches a game's fetched mod index. `columns` are what to sort the mods by, in order,
        /// with whether each is `descending`. The call returns the `skip`th and following mods
        /// that match, no more than `limit` of them, as summaries, and the count of all that match
        /// as its value.
        #[qinvokable]
        fn query(
            self: &ModIndex,
            game: &QString,
            query: &QString,
            columns: &QList_i32,
            descending: &QList_bool,
            skip: i32,
            limit: i32,
        ) -> *mut Call;

        /// Looks a mod up in a game's fetched mod index. The call returns it, with all of its
        /// versions, or null if it isn't there.
        #[qinvokable]
        fn get(self: &ModIndex, game: &QString, owner: &QString, name: &QString) -> *mut Call;
    }
}

use core::pin::Pin;

use cxx_qt_lib::{QList, QString, QVariant};

use super::call::{self, Call};
use super::mods::{ModInfo, ModKey};
use crate::mod_index::{self, SortColumn};
use crate::util::search::SortOption;
use crate::Reqwest;
use qobject::ModSortColumn;

#[derive(Default)]
pub struct ModIndexRust;

impl qobject::ModIndex {
    fn fetch(&self, game: &QString, refresh: bool) -> *mut Call {
        let game = game.to_string();
        call::spawn_with_task(move |task| async move {
            let app = super::app();
            mod_index::fetch_mod_index(
                Some(app),
                &app.state::<Reqwest>(),
                &game,
                refresh,
                Some(task),
            )
            .await
        })
    }

    fn query(
        &self,
        game: &QString,
        query: &QString,
        columns: &QList<i32>,
        descending: &QList<bool>,
        skip: i32,
        limit: i32,
    ) -> *mut Call {
        let (game, query) = (game.to_string(), query.to_string());
        let mut sort = Vec::<SortOption<SortColumn>>::new();
        for (&column, &descending) in columns.iter().zip(descending.iter()) {
            // QML can pass any number
            let Ok(column) = SortColumn::try_from(ModSortColumn { repr: column }) else {
                continue;
            };
            // each column can only be sorted by once
            if !sort.iter().any(|option| option.column == column) {
                sort.push(SortOption { column, descending });
            }
        }
        let skip = usize::try_from(skip).unwrap_or(0);
        let limit = usize::try_from(limit).unwrap_or(0);
        call::spawn(async move {
            let index = mod_index::read_mod_index(&game).await?;
            let results = mod_index::query_mod_index(&index, &query, &sort)?;
            let mods = results
                .iter()
                .skip(skip)
                .take(limit)
                .map(|&(position, _)| ModInfo::summary(index.get(position)))
                .collect();
            anyhow::Ok(Page {
                count: results.len(),
                mods,
            })
        })
    }

    fn get(&self, game: &QString, owner: &QString, name: &QString) -> *mut Call {
        let (game, key) = (game.to_string(), ModKey::of(owner, name));
        call::spawn(async move {
            let index = mod_index::read_mod_index(&game).await?;
            let found = mod_index::get_one_from_mod_index(&index, key.id()).await?;
            anyhow::Ok(found.map(ModInfo::listing))
        })
    }
}

/// A page of the mods that match a query, as summaries, with the count of all that match as the
/// call's value.
struct Page {
    count: usize,
    mods: Vec<ModInfo>,
}

impl call::Output for Page {
    fn return_from(self, mut call: Pin<&mut Call>) {
        call.as_mut()
            .set_value(QVariant::from(&(self.count as f64)));
        self.mods.return_from(call);
    }
}

impl TryFrom<ModSortColumn> for SortColumn {
    type Error = ();

    fn try_from(column: ModSortColumn) -> Result<Self, ()> {
        Ok(match column {
            ModSortColumn::Relevance => Self::Relevance,
            ModSortColumn::Downloads => Self::Downloads,
            ModSortColumn::Name => Self::Name,
            ModSortColumn::Owner => Self::Owner,
            ModSortColumn::Size => Self::Size,
            _ => return Err(()),
        })
    }
}
