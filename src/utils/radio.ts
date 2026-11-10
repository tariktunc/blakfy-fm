import type { VoiceChannel } from "discord.js";
import type { SearchResult } from "lavalink-client";
import type { Context, Lavamusic } from "../structures/index";

/** Local radio-proxy container: converts live streams to MP3 for Lavalink. */
const PROXY = "http://radio-proxy:8000";

/**
 * Live radio stations. Served as MP3 by the local radio-proxy container,
 * because Lavalink cannot decode these stations' streams directly.
 */
export const RADIO_STATIONS = {
	kafaradyo: { name: "Kafa Radyo", url: `${PROXY}/kafaradyo.mp3`, site: "https://kafaradyo.com" },
	cnnturk: { name: "CNN Türk Radyo", url: `${PROXY}/cnnturk.mp3`, site: "https://www.cnnturk.com/radyo" },
	trthaber: { name: "TRT Haber Radyo", url: `${PROXY}/trthaber.mp3`, site: "https://www.trthaber.com" },
	haberturk: { name: "Habertürk (TV sesi)", url: `${PROXY}/haberturk.mp3`, site: "https://www.haberturk.com/canliyayin" },
	damarfm: { name: "Damar Türk FM", url: `${PROXY}/damarfm.mp3`, site: "https://www.damarturkfm.com" },
	lofi: { name: "Lofi (FluxFM Chillhop)", url: `${PROXY}/lofi.mp3`, site: "https://www.fluxfm.de" },
	paradise: { name: "Radio Paradise", url: `${PROXY}/paradise.mp3`, site: "https://radioparadise.com" },
	rock: { name: "Radio Paradise Rock", url: `${PROXY}/rock.mp3`, site: "https://radioparadise.com" },
	mellow: { name: "Radio Paradise Mellow", url: `${PROXY}/mellow.mp3`, site: "https://radioparadise.com" },
	groove: { name: "SomaFM Groove Salad", url: `${PROXY}/groove.mp3`, site: "https://somafm.com/groovesalad" },
	kexp: { name: "KEXP Seattle", url: `${PROXY}/kexp.mp3`, site: "https://www.kexp.org" },
	jazz: { name: "Jazz24", url: `${PROXY}/jazz.mp3`, site: "https://www.jazz24.org" },
	swisspop: { name: "Radio Swiss Pop", url: `${PROXY}/swisspop.mp3`, site: "https://www.radioswisspop.ch" },
} as const;

/** Stations listed by `!radyo` (foreign music radios). */
export const MUSIC_STATIONS: RadioKey[] = ["paradise", "rock", "mellow", "groove", "kexp", "jazz", "swisspop", "lofi"];

export type RadioKey = keyof typeof RADIO_STATIONS;

export async function playRadio(client: Lavamusic, ctx: Context, key: RadioKey): Promise<any> {
	const s = RADIO_STATIONS[key];
	return await playStream(client, ctx, s.name, s.url, s.site);
}

/** Plays the audio of a live Kick channel. Accepts "slug" or a kick.com URL. */
export async function playKick(client: Lavamusic, ctx: Context, input: string): Promise<any> {
	const slug = (input ?? "").trim().replace(/\/+$/, "").split("/").pop() ?? "";
	if (!/^[A-Za-z0-9_-]{1,40}$/.test(slug)) {
		return await ctx.sendMessage({
			embeds: [client.embed().setColor(client.color.red).setDescription("Kick kanal adı geçersiz. Örnek: `!kick kanaladi`")],
		});
	}
	return await playStream(client, ctx, `Kick: ${slug}`, `${PROXY}/kick/${slug}.mp3`, `https://kick.com/${slug}`, "Kanal yayında değil ya da bulunamadı.");
}

/**
 * Joins the caller's voice channel and plays a live stream immediately.
 * Clears the queue and replaces whatever is playing.
 */
export async function playStream(
	client: Lavamusic,
	ctx: Context,
	name: string,
	url: string,
	site: string,
	failText = "yayınına ulaşılamadı.",
): Promise<any> {
	await ctx.sendDeferMessage(`📻 ${name} açılıyor...`);

	const memberVoiceChannel = (ctx.member as any).voice.channel as VoiceChannel;
	let player = client.manager.getPlayer(ctx.guild.id);
	if (!player)
		player = client.manager.createPlayer({
			guildId: ctx.guild.id,
			voiceChannelId: memberVoiceChannel.id,
			textChannelId: ctx.channel.id,
			selfMute: false,
			selfDeaf: true,
			vcRegion: memberVoiceChannel.rtcRegion!,
		});
	if (!player.connected) await player.connect();

	let response: SearchResult | undefined;
	try {
		response = (await player.search({ query: url }, ctx.author)) as SearchResult;
	} catch {
		response = undefined;
	}
	const track = response?.tracks?.[0];
	if (!track) {
		const text = failText.startsWith("yayın") ? `${name} ${failText}` : failText;
		return await ctx.editMessage({
			content: "",
			embeds: [client.embed().setColor(client.color.red).setDescription(text)],
		});
	}

	// Stream metadata has no title; show the station name instead.
	(track.info as any).title = name;
	(track.info as any).author = "Canlı yayın";
	(track.info as any).uri = site;

	player.set("autoplay", false);
	player.queue.tracks.splice(0, player.queue.tracks.length);
	await player.play({ clientTrack: track, paused: false });

	return await ctx.editMessage({
		content: "",
		embeds: [client.embed().setColor(client.color.main).setDescription(`📻 **${name}** çalıyor.`)],
	});
}
