import * as path from "@std/path";

const mainModuleUrl = Deno.mainModule;
export const mainModuleDir = path.dirname(path.fromFileUrl(mainModuleUrl));
export const projectRootDir = path.dirname(path.dirname(mainModuleDir));
console.error(`projectRootDir: ${projectRootDir}`);
