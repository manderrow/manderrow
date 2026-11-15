import * as path from "node:path";

export const projectRootDir = path.join(import.meta.dir, "..", "..", "..");
console.error(`projectRootDir: ${projectRootDir}`);
