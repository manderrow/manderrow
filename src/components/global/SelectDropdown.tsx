import styles from "./SelectDropdown.module.css";

interface Option {
  name: string;
  value: any;
}

export default function SelectDropdown(props: { options: Option[] }) {
  return <div class={styles.select}></div>;
}
