import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Manderrow

// A searchable, paginated list of mods beside a view of the focused mod, ported from ModList in
// ModList.tsx.
Item {
    id: modList
    objectName: "modList"

    property string game
    property var installContext: null
    /// `(game, query, sort) => Promise<{ count, mods: page => Promise<mods>, get: id => mod }>`.
    /// Replacing it queries the mods again.
    property var fetcher: null
    /// Reloads the mods. May return a promise.
    property var refresh: function () {}
    property bool isLoading: false
    property var progress
    /// Controls shown at the end of the line above the list.
    property Component trailingControls: null
    /// Whether mods can be selected for bulk actions.
    property bool multiselect: false

    readonly property int modsPerPage: 50

    property bool profileSortOrder: false
    property string query: ""
    property var sort: [
        {
            column: Api.modSortColumn.relevance,
            descending: true
        },
        {
            column: Api.modSortColumn.downloads,
            descending: true
        },
        {
            column: Api.modSortColumn.name,
            descending: false
        },
        {
            column: Api.modSortColumn.owner,
            descending: false
        },
        {
            column: Api.modSortColumn.size,
            descending: true
        }
    ]

    // the query results
    property var queried: null
    property var queriedError: null
    property bool queriedLoading: false
    property int _queryGeneration: 0

    // the focused mod, shown in the mod view
    property var focusedModId: null
    property var focusedMod: null
    property int _focusGeneration: 0
    /// Identifies the focused mod. Its object is replaced when the results are, which leaves the
    /// key unchanged.
    readonly property string _focusedKey: focusedMod ? focusedMod.owner + "/" + focusedMod.name : ""

    // the loaded pages of mods, which `rows` index
    property var mods: []
    property int _nextPage: 0
    property bool _end: true
    property bool _pageLoading: false

    function requery() {
        if (!fetcher)
            return;
        const generation = ++_queryGeneration;
        queriedLoading = true;
        Promise.resolve(fetcher(game, query, sort)).then(result => {
            // the component may have been destroyed by the time this runs
            if (!modList || generation !== modList._queryGeneration)
                return;
            modList.queriedError = null;
            modList.queried = result;
            modList.queriedLoading = false;
        }, e => {
            if (!modList || generation !== modList._queryGeneration)
                return;
            console.error(e);
            modList.queriedError = e;
            modList.queriedLoading = false;
        });
    }

    onFetcherChanged: requery()
    // searched once typing pauses, rather than for every keystroke
    onQueryChanged: queryDebounce.restart()
    onSortChanged: requery()

    Timer {
        id: queryDebounce
        interval: 150
        onTriggered: modList.requery()
    }

    onQueriedChanged: {
        rows.clear();
        // emptied in place: the rows just removed are only destroyed later, and replacing the
        // array would update them meanwhile, with no mod left at their index
        mods.length = 0;
        _nextPage = 0;
        _end = false;
        loadMore();
        updateFocusedMod();
    }
    onFocusedModIdChanged: updateFocusedMod()

    /// Resolves the focused mod in the current results, and unfocuses it if it's gone.
    function updateFocusedMod() {
        const generation = ++_focusGeneration;
        const id = focusedModId;
        if (id === null || queried === null) {
            focusedMod = null;
            return;
        }
        Promise.resolve(queried.get(id)).then(mod => {
            if (!modList || generation !== modList._focusGeneration)
                return;
            if (mod) {
                modList.focusedMod = mod;
            } else {
                modList.focusedMod = null;
                modList.focusedModId = null;
            }
        }, e => console.error(e));
    }

    function loadMore() {
        if (_end || _pageLoading || queried === null)
            return;
        _pageLoading = true;
        const result = queried;
        const page = _nextPage++;
        Promise.resolve(result.mods(page)).then(pageMods => {
            if (!modList || result !== modList.queried)
                return;
            modList._pageLoading = false;
            if (pageMods.length === 0) {
                modList._end = true;
                return;
            }
            const start = modList.mods.length;
            // appended in place: the rows already loaded keep their mods, so they aren't updated
            modList.mods.push(...pageMods);
            rows.append(pageMods.map((_, i) => ({ i: start + i })));
            // keep loading until the list is filled
            Qt.callLater(list.maybeLoadMore);
        }, e => {
            if (!modList)
                return;
            modList._pageLoading = false;
            modList._end = true;
            Ui.window.reportError(e);
        });
    }

    // bulk selection of installed mods, ported from createMultiselectableList
    property var selected: ({})
    property var selectedOrder: []
    property int pivotIndex: -1

    function _key(mod) {
        return `${mod.owner}-${mod.name}`;
    }
    readonly property var selectedMods: {
        const out = [];
        for (const key of selectedOrder) {
            if (selected[key])
                out.push(selected[key]);
        }
        return out;
    }
    function isSelected(mod) {
        return !!selected[_key(mod)];
    }
    function _setSelection(entries) {
        const map = {};
        const order = [];
        for (const mod of entries) {
            map[_key(mod)] = { owner: mod.owner, name: mod.name, version: mod.version.version_number };
            order.push(_key(mod));
        }
        selected = map;
        selectedOrder = order;
    }
    function toggleSelected(mod, index) {
        const key = _key(mod);
        const map = Object.assign({}, selected);
        let order = selectedOrder.slice();
        if (map[key]) {
            delete map[key];
            order = order.filter(k => k !== key);
        } else {
            map[key] = { owner: mod.owner, name: mod.name, version: mod.version.version_number };
            order.push(key);
        }
        selected = map;
        selectedOrder = order;
        pivotIndex = index;
    }
    function shiftSelect(mod, index) {
        const items = installContext.installed;
        const pivot = pivotIndex >= 0 ? items[pivotIndex] : undefined;
        if (pivot === undefined || Util.modIdEquals(pivot, mod))
            return;
        const pivotAt = items.findIndex(p => Util.modIdEquals(p, pivot));
        _setSelection(items.slice(Math.min(pivotAt, index), Math.max(pivotAt, index) + 1));
    }
    function deselect(mod) {
        const key = _key(mod);
        const map = Object.assign({}, selected);
        delete map[key];
        selected = map;
        selectedOrder = selectedOrder.filter(k => k !== key);
    }
    function clearSelection() {
        selected = {};
        selectedOrder = [];
        pivotIndex = -1;
    }

    // selectors stay visible until the list is first hovered and left, to hint at them
    property int tutorialState: 0 // INIT, HOVERED, LEFT

    ListModel {
        id: rows
    }

    // grid-template-columns: minmax(0, 5fr) minmax(0, 3fr), in the page's padding, which the mod
    // view extends into (margin-right: -1em)
    Item {
        anchors.fill: parent

        ColumnLayout {
            id: listColumn
            width: (parent.width - Theme.rem) * 5 / 8
            height: parent.height
            spacing: 0.5 * Theme.rem

            ModSearch {
                Layout.fillWidth: true
                query: modList.query
                sort: modList.sort
                profileSortOrder: modList.profileSortOrder
                onQueryEdited: query => modList.query = query
                onSortEdited: sort => modList.sort = sort
                onProfileSortOrderEdited: order => modList.profileSortOrder = order
            }

            // display: flex. Its messages and trailing controls shrink as flex items do, with their
            // words wrapped, rather than widening the list's column.
            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                // as wide as its text, or narrower, wrapped, when the line is short of space.
                // Layouts round widths to whole pixels, so its width is rounded up, or it would
                // wrap for want of a fraction of one.
                component LineText: Text {
                    id: lineText
                    Layout.fillWidth: true
                    Layout.maximumWidth: Math.ceil(implicitWidth)
                    Layout.minimumWidth: Util.longestWordWidth(lineMetrics, text)
                    Layout.topMargin: 8
                    Layout.bottomMargin: 8
                    wrapMode: Text.WordWrap
                    color: Theme.neutral05
                    font.family: Theme.font
                    font.pixelSize: Theme.rem

                    FontMetrics {
                        id: lineMetrics
                        font: lineText.font
                    }
                }

                LineText {
                    visible: modList.isLoading || modList.queriedLoading
                    text: I18n.t("modlist.fetching_msg")
                }
                ProgressBar {
                    visible: modList.isLoading || modList.queriedLoading
                    Layout.fillWidth: true
                    Layout.maximumWidth: 12 * Theme.rem
                    progress: modList.progress
                }

                ErrorIndicator {
                    visible: !modList.isLoading && !modList.queriedLoading && modList.queriedError !== null
                    message: "Query failed"
                    err: modList.queriedError
                    reset: () => Promise.resolve(modList.refresh()).then(() => modList.requery())
                }

                LineText {
                    visible: !modList.isLoading && !modList.queriedLoading && modList.queriedError === null && modList.queried !== null
                    text: I18n.t("modlist.discovered_msg", {
                        count: Util.formatNumber(modList.queried ? modList.queried.count : 0)
                    })
                }
                AsyncButton {
                    visible: !modList.isLoading && !modList.queriedLoading && modList.queriedError === null && modList.queried !== null
                    variant: "plain"
                    iconName: "refresh-cw"
                    btn.padAll: 8
                    btn.activeBgColor: Theme.primary40
                    btn.width: btn.height
                    action: () => modList.refresh()
                }

                // margin-left: auto
                Item {
                    Layout.fillWidth: true
                    Layout.minimumWidth: trailing.item && trailing.item.minimumWidth !== undefined ? trailing.item.minimumWidth : trailing.implicitWidth
                    Layout.preferredWidth: Math.ceil(trailing.implicitWidth)
                    implicitHeight: trailing.implicitHeight

                    Loader {
                        id: trailing
                        anchors.right: parent.right
                        width: Math.min(implicitWidth, parent.width)
                        sourceComponent: modList.trailingControls
                    }
                }
            }

            ListView {
                id: list
                objectName: "modListView"
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                // padding-right: 0.5em, beside the scroll bar
                rightMargin: 0.5 * Theme.rem + listBar.gutter
                boundsBehavior: Flickable.StopAtBounds
                ScrollBar.vertical: AppScrollBar {
                    id: listBar
                }
                WheelScroll {}
                // one stop in the Tab order, whose items the arrow keys move between
                FocusGroup {
                    view: list
                }
                visible: modList.queriedError === null
                model: rows

                function maybeLoadMore() {
                    if (contentY + height > contentHeight - 200)
                        modList.loadMore();
                }
                onContentYChanged: maybeLoadMore()
                onHeightChanged: maybeLoadMore()

                HoverHandler {
                    onHoveredChanged: {
                        if (hovered && modList.tutorialState === 0)
                            modList.tutorialState = 1;
                        else if (!hovered && modList.tutorialState === 1)
                            modList.tutorialState = 2;
                    }
                }

                // border-right
                Rectangle {
                    parent: list
                    anchors.right: parent.right
                    width: 1
                    height: parent.height
                    color: Theme.neutral80
                    z: 1
                }

                property int hotIndex: -1

                delegate: ModListItem {
                    id: entry

                    required property int i
                    required property int index

                    width: list.width - list.rightMargin
                    mod: modList.mods[i]
                    installContext: modList.installContext
                    focused: Util.modIdEquals(modList.focusedModId, mod)
                    selectable: modList.multiselect
                    selected: modList.multiselect && modList.isSelected(mod)
                    forceSelectorVisibility: modList.selectedMods.length !== 0 || modList.tutorialState < 2
                    previousHot: index > 0 && list.hotIndex === index - 1
                    onHotChanged: {
                        if (hot)
                            list.hotIndex = index;
                        else if (list.hotIndex === index)
                            list.hotIndex = -1;
                    }
                    onFocusToggled: modList.focusedModId = focused ? null : { owner: mod.owner, name: mod.name }
                    onSelectToggled: modList.toggleSelected(mod, index)
                    onShiftClicked: modList.shiftSelect(mod, index)
                    onUninstalled: modList.deselect(mod)
                }

                footer: Text {
                    visible: !modList._end
                    width: list.width
                    padding: 0.5 * Theme.rem
                    text: I18n.t("global.phrases.loading") + "..."
                    color: Theme.neutral05
                    font.family: Theme.font
                    font.pixelSize: Theme.rem
                }
            }
        }

        // the mod view, or bulk actions for the selection
        Item {
            x: listColumn.width
            width: parent.width - x
            height: parent.height

            Item {
                anchors.fill: parent
                anchors.leftMargin: 0.75 * Theme.rem

                // a mod view for each mod, so none of one mod's state carries over to the next
                Repeater {
                    model: modList._focusedKey !== "" ? [modList._focusedKey] : []

                    ModView {
                        anchors.fill: parent
                        mod: modList.focusedMod
                        gameId: modList.game
                        installContext: modList.installContext
                        onCloseRequested: modList.focusedModId = null
                    }
                }

                Loader {
                    anchors.fill: parent
                    active: modList.focusedMod === null && modList.multiselect && modList.selectedMods.length !== 0
                    sourceComponent: BulkActions {
                        mods: modList.selectedMods
                        installContext: modList.installContext
                        onRemoveRequested: mod => modList.deselect(mod)
                        onCancelRequested: modList.clearSelection()
                    }
                }

                // as wide as its text, or as the column, with its text wrapped
                ColumnLayout {
                    visible: modList.focusedMod === null && !(modList.multiselect && modList.selectedMods.length !== 0)
                    anchors.centerIn: parent
                    anchors.verticalCenterOffset: -1.5 * Theme.rem
                    width: Math.min(Math.ceil(implicitWidth), parent.width)
                    spacing: 0.25 * 1.5 * Theme.rem

                    Text {
                        Layout.fillWidth: true
                        horizontalAlignment: Text.AlignHCenter
                        wrapMode: Text.WordWrap
                        text: I18n.t("modlist.modview.no_mod_selected_title")
                        color: Theme.neutral20
                        font.family: Theme.font
                        font.pixelSize: 1.5 * Theme.rem
                    }
                    Text {
                        Layout.fillWidth: true
                        horizontalAlignment: Text.AlignHCenter
                        wrapMode: Text.WordWrap
                        text: I18n.t("modlist.modview.no_mod_selected_subtitle")
                        color: Theme.neutral30
                        font.family: Theme.font
                        font.pixelSize: Theme.rem
                    }
                }
            }
        }
    }
}
