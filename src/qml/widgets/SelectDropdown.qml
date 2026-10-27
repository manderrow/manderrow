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
    // max-height: 50dvh
    panelMaxHeight: Ui.window ? Ui.window.height * 0.5 : 400

    signal changed(var value, bool selected)

    label: {
        if (preset !== "")
            return preset;
        const selected = options.find(option => option.selected);
        if (selected !== undefined)
            return selected.label;
        return fallback !== "" ? fallback : Strings.global.select_dropdown.default_fallback;
    }

    SelectList {
        options: root.options
        fontSize: root.optionFontSize
        itemPadding: root.optionPadding
        onChanged: (value, selected) => root.changed(value, selected)
    }
}
