const cheerio = require('cheerio');
const games = require("../../src-tauri/src/games/games.json");
const fs = require("fs");
const { type } = require('os');

const gameStoreIds = games.map(gameInfo => gameInfo.storePlatformMetadata[0].storeIdentifier);

const urls = games.map(gameInfo => gameInfo.thunderstoreUrl);

const gameIds = [];

for (let i = 0; i < urls.length; i++) {
    const url = new URL(urls[i]);
    const thunderstoreId = url.pathname.slice(1).split("/")[1];

    gameIds.push(thunderstoreId);
}

//const gameIds = games.map(gameInfo => gameInfo.id);

console.dir(gameIds, {depth: null, 'maxArrayLength': null});

console.dir(gameStoreIds, {depth: null, 'maxArrayLength': null});

async function scrape() {

    const gameReviews = await Promise.all(
        gameStoreIds.map(async (id) => {
            try{const $ = await cheerio.fromURL("https://store.steampowered.com/app/" + id);

                const $reviewCount = $("meta[itemprop=reviewCount]").attr("content");
    
                const finalInfo = {
                    reviewCount: $reviewCount,
                }
    
                return finalInfo;
            } catch {console.error(id)}
            
        })
    )

    console.dir(gameReviews, {depth: null, 'maxArrayLength': null});

    let gameFinal = {}

    for (let i = 0; i < gameReviews.length; i++) {
        console.log(gameIds[i]);
        console.log(gameReviews[i]);

        gameFinal[gameIds[i]] = parseInt(gameReviews[i]?.reviewCount || -1);
    }

    console.dir(gameFinal, {depth: null, 'maxArrayLength': null});

    const gameJSON = JSON.stringify(gameFinal, undefined, 2);

    fs.writeFile("gameReviews.json", gameJSON,function(err, result) {
        if(err) console.log('error', err);
    });
}

scrape();