"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.handleTranslation = handleTranslation;
const LyricsFetcher_1 = require("./LyricsFetcher");
const LrcLibSource_1 = require("./Sources/LrcLibSource");
const NetEaseMusicSource_1 = require("./Sources/NetEaseMusicSource");
const QQMusicSource_1 = require("./Sources/QQMusicSource");
const PlaybackStateUpdater_1 = require("./PlaybackStateUpdater");
const PlaybackState_1 = require("./PlaybackState");
const StatusChanger_1 = require("./StatusChanger");
const Debug_1 = require("./Debug");
const Server_1 = require("./Panel/Server");
const Settings_1 = require("./Settings");
const Updater_1 = require("./Updater");
const uuid_1 = require("uuid");
// ─── Bootstrap ────────────────────────────────────────────────────────────────
Settings_1.Settings.load();
if (Settings_1.Settings.update.enableAutoupdate) {
    Updater_1.Updater.tryUpdate()
        .then(init)
        .catch((e) => {
        Debug_1.Debug.write(`Auto-update failed: ${e.stack}`);
        init();
    });
}
else {
    init();
}
function handleTranslation(playbackState) {
    return __awaiter(this, void 0, void 0, function* () {
        var _a;
        const line = playbackState.currentLine;
        if (line &&
            line.text &&
            ((_a = Settings_1.Settings.translation) === null || _a === void 0 ? void 0 : _a.enableTranslation) &&
            !line.textTranslated) {
            try {
                const { translateLyrics } = yield Promise.resolve().then(() => __importStar(require("./Translate")));
                const translated = yield translateLyrics(line.text, Settings_1.Settings.translation.translationLanguage);
                line.textTranslated = translated;
            }
            catch (err) {
                console.error("Error translating lyrics:", err);
            }
        }
    });
}
// ─── Init ─────────────────────────────────────────────────────────────────────
function init() {
    // Ensure a stable UUID exists for this installation
    if (!Settings_1.Settings.credentials.uuid) {
        Settings_1.Settings.credentials.uuid = (0, uuid_1.v4)();
        Settings_1.Settings.save();
    }
    // Lyrics sources — tried in order, first success wins
    const lyricsFetcher = new LyricsFetcher_1.LyricsFetcher();
    lyricsFetcher.addSource(new LrcLibSource_1.LrcLibSource()); // best synced-LRC coverage
    lyricsFetcher.addSource(new NetEaseMusicSource_1.NetEaseMusicSource()); // large Chinese + global catalogue
    lyricsFetcher.addSource(new QQMusicSource_1.QQMusicSource()); // last resort
    const playbackState = new PlaybackState_1.PlaybackState();
    const playbackStateUpdater = new PlaybackStateUpdater_1.PlaybackStateUpdater(playbackState, lyricsFetcher);
    const statusChanger = new StatusChanger_1.StatusChanger(playbackState);
    // Poll Spotify (via Discord token) every 2 s to detect song changes
    setInterval(() => playbackStateUpdater.update(), 2000);
    // 60 fps loop: advance local progress counter + fire status updates
    let lastTick = Date.now();
    setInterval(() => {
        const now = Date.now();
        const delta = now - lastTick;
        lastTick = now;
        playbackState.songProgress += delta;
        statusChanger.changeStatus();
        if (playbackState.ended)
            statusChanger.songChanged();
        handleTranslation(playbackState); // Para ver primero si termino la cancion y no traducir
        console.clear();
        console.log(`  Song:    ${playbackState.songName || "—"}\n` +
            `  Artist:  ${playbackState.songAuthor || "—"}\n` +
            `  Time:    ${statusChanger.formatSeconds(Math.floor(playbackState.songProgress / 1000))}\n` +
            `  Lyrics:  ${playbackState.currentLine ? (playbackState.currentLine.textTranslated || playbackState.currentLine.text) : "—"}\n` +
            `  Source:  ${lyricsFetcher.lastFetchedFrom}`);
    }, 1000 / 60);
    (0, Server_1.startServer)();
}
// ─── Error handling ───────────────────────────────────────────────────────────
process.on("uncaughtException", (e) => {
    var _a;
    Debug_1.Debug.write(`${e.stack}\n${(_a = e.cause) !== null && _a !== void 0 ? _a : ""}`);
    // Network errors are transient — keep running
    if (!e.message.includes("fetch failed"))
        process.exit(1);
});
