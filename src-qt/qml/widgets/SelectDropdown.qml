import QtQuick
import Manderrow

// A dropdown for choosing among options, ported from SelectDropdown.tsx. See SelectList for the
// shape of `options`.
Dropdown {
    id: root

    property var options: []
    /// When set, the trigger always shows this text instead of the selected option.
    property string preset
    property string fallback
    property real optionFontSize: Theme.rem
    property real optionPadding: 0.3 * optionFontSize

    signal changed(var value, bool selected)

    label: {
        if (preset !== "")
            return preset;
        const selected = options.find(option => option.selected);
        if (selected !== undefined)
            return selected.label;
        return fallback !== "" ? fallback : I18n.t("global.select_dropdown.default_fallback");
    }

    contentWidth: list.implicitWidth
    contentHeight: list.contentHeight

    onAboutToShow: Qt.callLater(list.scrollToSelected)

    SelectList {
        id: list
        width: root.panel.availableWidth
        height: root.panel.availableHeight
        options: root.options
        fontSize: root.optionFontSize
        itemPadding: root.optionPadding
        onChanged: (value, selected) => root.changed(value, selected)
    }
}
