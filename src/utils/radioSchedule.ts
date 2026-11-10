import type { Lavamusic } from "../structures/index";
import logger from "../structures/Logger";
import { RADIO_STATIONS } from "./radio";

/**
 * Scheduled Kafa Radyo for Nihat Sirdar's programs (Blakfy server).
 *
 * - During a program the bot joins the "Blakfy" voice channel and plays
 *   Kafa Radyo, even if the channel is empty, and stays there until the end.
 * - It never interrupts other music: if the bot is already playing, it waits.
 * - Stateless across restarts: on every start it checks the clock, so if the
 *   PC/bot comes up in the middle of a program it joins right away.
 * - When the program ends it leaves, but only if the scheduled radio is still on.
 * - If someone sends it away (people in the channel, bot removed) it does not
 *   rejoin for the rest of that program.
 *
 * Schedule source: kafaradyo.com yayin akisi (checked 2026-11-10).
 */
const GUILD_ID = "420151446290563082";
const VOICE_CHANNEL_ID = "1113567491088719872"; // voice: Blakfy
const TEXT_CHANNEL_ID = "1439982697593176337"; // text: #music
const FLAG = "scheduledRadio"; // player data key, value = program instance key

interface Slot {
	days: number[]; // 0 = Sunday ... 6 = Saturday
	start: number; // minutes after midnight, Turkey time
	end: number;
	name: string;
}

const SLOTS: Slot[] = [
	{ days: [1, 2, 3, 4, 5], start: 7 * 60, end: 9 * 60, name: "Nihat'la Muhabbet" },
	{ days: [1, 2, 3, 4, 5], start: 18 * 60, end: 20 * 60, name: "Nihat'la Sivrisinek" },
	{ days: [6], start: 17 * 60, end: 19 * 60, name: "Nihat'la Sivrisinek" },
];

// Optional test slot, every day: RADIO_TEST_SLOT="HH:MM-HH:MM" (Turkey time).
const test = /^(\d{2}):(\d{2})-(\d{2}):(\d{2})$/.exec(process.env.RADIO_TEST_SLOT ?? "");
if (test) {
	SLOTS.push({
		days: [0, 1, 2, 3, 4, 5, 6],
		start: Number(test[1]) * 60 + Number(test[2]),
		end: Number(test[3]) * 60 + Number(test[4]),
		name: "Test",
	});
}

/** Turkey is fixed UTC+3 (no DST since 2016). */
function currentProgram(): { slot: Slot; key: string } | null {
	const d = new Date(Date.now() + 3 * 60 * 60 * 1000);
	const day = d.getUTCDay();
	const minutes = d.getUTCHours() * 60 + d.getUTCMinutes();
	const slot = SLOTS.find((s) => s.days.includes(day) && minutes >= s.start && minutes < s.end);
	return slot ? { slot, key: `${d.toISOString().slice(0, 10)}-${slot.start}` } : null;
}

/**
 * True while the scheduled radio is playing in its program window.
 * Used by VoiceStateUpdate to skip the "leave empty channel" rule.
 */
export function isScheduledRadioHolding(client: Lavamusic, guildId: string): boolean {
	if (guildId !== GUILD_ID) return false;
	const program = currentProgram();
	const player = client.manager.getPlayer(guildId);
	return !!program && !!player && ownsPlayer(player) && player.get<string>(FLAG) === program.key;
}

/** True only while the player is still playing the scheduled radio (not user music). */
function ownsPlayer(player: any): boolean {
	return !!player.get(FLAG) && player.queue.current?.info?.title === RADIO_STATIONS.kafaradyo.name;
}

export function startRadioSchedule(client: Lavamusic): void {
	const station = RADIO_STATIONS.kafaradyo;
	let joinedKey: string | null = null; // program instance the bot joined for
	let blockedKey: string | null = null; // someone sent the bot away during it
	let busy = false;

	const tick = async () => {
		if (busy) return;
		busy = true;
		try {
			const guild = client.guilds.cache.get(GUILD_ID);
			if (!guild || !client.user) return;
			const program = currentProgram();
			const player = client.manager.getPlayer(GUILD_ID);
			const flag = player?.get<string>(FLAG);

			// Someone took over the player (stopped the radio or played music):
			// it is theirs now, never touch it.
			if (player && flag && !ownsPlayer(player)) player.set(FLAG, null);

			// Program over (or a different one started): leave if our radio is still on.
			const stale = !!player && ownsPlayer(player) && flag !== program?.key;
			if (stale) {
				await player!.destroy();
				logger.info("[RADIO] Program bitti, kanaldan çıkıldı.");
			}
			if (!program) {
				joinedKey = null;
				blockedKey = null;
				return;
			}
			if (player && !stale) return; // bot is busy (music or our radio)
			if (blockedKey === program.key) return;

			// We joined for this program and the bot is gone while people are in
			// the channel: someone sent it away. Respect that until the program ends.
			const humans = guild.voiceStates.cache.some(
				(s) => s.channelId === VOICE_CHANNEL_ID && s.id !== client.user!.id && s.member?.user?.bot !== true,
			);
			if (joinedKey === program.key && humans) {
				blockedKey = program.key;
				logger.info("[RADIO] Bot programdan çıkarıldı, bu program boyunca tekrar girmeyecek.");
				return;
			}

			const p = client.manager.createPlayer({
				guildId: GUILD_ID,
				voiceChannelId: VOICE_CHANNEL_ID,
				textChannelId: TEXT_CHANNEL_ID,
				selfMute: false,
				selfDeaf: true,
			});
			p.set(FLAG, program.key);
			if (!p.connected) await p.connect();
			const res = await p.search({ query: station.url }, client.user);
			const track = res?.tracks?.[0];
			if (!track) {
				await p.destroy();
				logger.warn("[RADIO] Kafa Radyo yayınına ulaşılamadı, 30 sn sonra tekrar denenecek.");
				return;
			}
			(track.info as any).title = station.name;
			(track.info as any).author = program.slot.name;
			(track.info as any).uri = station.site;
			p.set("autoplay", false);
			await p.play({ clientTrack: track, paused: false });
			joinedKey = program.key;
			logger.info(`[RADIO] ${program.slot.name} başladı, Kafa Radyo açıldı.`);
		} catch (error) {
			logger.error(`[RADIO] Zamanlayıcı hatası: ${error}`);
		} finally {
			busy = false;
		}
	};

	setTimeout(tick, 15_000);
	setInterval(tick, 30_000);
	logger.info("[RADIO] Kafa Radyo zamanlayıcısı açık.");
}
