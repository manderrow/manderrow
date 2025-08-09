import { Fa } from "solid-fa";
import { Progress } from "../../api/tasks";
import { faPause } from "@fortawesome/free-solid-svg-icons/index.js";

import styles from "./Progress.module.css";
import { JSX, Show } from "solid-js";
import { roundedNumberFormatter } from "../../utils";

// Display a simple linear progress bar
export function SimpleProgressIndicator(props: {
  /**
   * This property is not optional to discourage indeterminate usage.
   */
  progress: Progress | undefined;
  class?: JSX.HTMLAttributes<HTMLDivElement>["class"];
}) {
  const isIndeterminate = () => props.progress === undefined || props.progress.total === 0;

  return (
    <div
      role="progressbar"
      class={`${styles.simpleIndicator} ${props.class || ""}`}
      classList={{
        [styles.simpleIndicatorIndeterminate]: isIndeterminate(),
      }}
      style={
        isIndeterminate()
          ? {}
          : { "--percentage": `${((props.progress?.completed ?? 0) / (props.progress?.total ?? 1)) * 100}%` }
      }
      aria-valuenow={(props.progress?.total ?? 0) === 0 ? undefined : props.progress!.completed}
      aria-valuemin={0}
      aria-valuemax={props.progress?.total}
    >
      <Show when={props.progress}>
        {(progress) => (
          <p class={styles.simpleIndicator__text}>
            {progress().total === 0
              ? "0"
              : roundedNumberFormatter.format((progress().completed / progress().total) * 100)}
            %
          </p>
        )}
      </Show>
    </div>
  );
}

// Display a circular progress indicator with optional pause functionality
type PausableProps =
  | {
      pausable: true;
      paused: boolean;
      onPauseChange?: (paused: boolean) => void;
    }
  | {};
type CircularProgressProps = PausableProps & {
  progress: Progress;
  pausable?: boolean;
  noProgressText?: boolean;
};

export function CircularProgressIndicator(props: CircularProgressProps) {
  const progressPercentage = () =>
    props.progress.total == 0 ? 0 : (props.progress.completed / props.progress.total) * 100;

  return (
    <div data-progress-indicator>
      <svg viewBox="0 0 200 200" xmlns="http://www.w3.org/2000/svg" class={styles.circularSvg}>
        <circle r="90" cx="100" cy="100" class={styles.progressBg}></circle>
        <circle
          r="90"
          cx="100"
          cy="100"
          transform="rotate(-90 100 100)"
          pathLength="100"
          stroke-dasharray="100"
          stroke-dashoffset={100 - progressPercentage()}
          class={styles.progressBar}
        ></circle>
        <foreignObject width="100%" height="100%">
          <div>
            <Show when={props.pausable}>
              <button class={styles.pauseBtn}>
                <Fa icon={faPause} />
              </button>
            </Show>
            <Show when={!props.noProgressText && !props.pausable}>
              <span class={styles.progressText} data-inner>
                {Math.round(progressPercentage())}%
              </span>
            </Show>
          </div>
        </foreignObject>
      </svg>
      <Show when={!props.noProgressText && props.pausable}>
        <p class={styles.progressText} data-outer>
          {Math.round(progressPercentage())}%
        </p>
      </Show>
    </div>
  );
}
