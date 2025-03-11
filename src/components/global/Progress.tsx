import { Progress } from "../../api/tasks";

export function SimpleProgressIndicator(props: { progress: Progress }) {
  // this complains about taking null but expecting undefined, but if we give it undefined it throws an error about the value being non-finite
  return (
    <progress
      value={props.progress.total_progress === 0 ? null : props.progress.completed_progress}
      max={props.progress.total_progress}
    />
  );
}
