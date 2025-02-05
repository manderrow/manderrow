import styles from "./SelectDropdown.module.css";

interface Option {
  name: string;
  value: any;
}

type LabelTextValue = {
  labelText: "value";
};
type LabelTextPreset = {
  labelText: "preset";
  preset: string;
};
type LabelText = LabelTextValue & LabelTextPreset;

export default function SelectDropdown(props: { options: Option[] }) {
  return <div class={styles.select}></div>;
}
