import { createResource } from "solid-js";
import { getGames } from "./api";

export const [games] = createResource(getGames);