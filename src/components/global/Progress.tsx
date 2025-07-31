import { Progress } from "../../api/tasks";

export function SimpleProgressIndicator(props: {
  /**
   * This property is not optional to discourage indeterminate usage.
   */
  progress: Progress | undefined;
}) {
  return (
    <progress
      // @ts-ignore: this complains about taking null but expecting undefined, but if we give it
      //             undefined it throws an error about the value being non-finite
      value={(props.progress?.total ?? 0) === 0 ? null : props.progress!.completed}
      max={props.progress?.total}
    />
  );
}
