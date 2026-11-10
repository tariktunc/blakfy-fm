import { Command, type Context, type Lavamusic } from "../../structures/index";
import { Connect, EmbedLinks, ReadMessageHistory, SendMessages, Speak, ViewChannel } from "../../utils/Permissions";
import { MUSIC_STATIONS, playRadio, RADIO_STATIONS, type RadioKey } from "../../utils/radio";

export default class Radyo extends Command {
	constructor(client: Lavamusic) {
		super(client, {
			name: "radyo",
			description: {
				content: "Yabancı müzik radyolarından birini çalar, isim yazılmazsa listeyi gösterir",
				examples: ["radyo", "radyo paradise", "radyo jazz"],
				usage: "radyo [istasyon]",
			},
			category: "radio",
			aliases: ["radio"],
			cooldown: 3,
			args: false,
			vote: false,
			player: { voice: true, dj: false, active: false, djPerm: null },
			permissions: {
				dev: false,
				client: [SendMessages, ReadMessageHistory, ViewChannel, EmbedLinks, Connect, Speak],
				user: [],
			},
			slashCommand: true,
			options: [
				{
					name: "istasyon",
					description: "Çalınacak radyo",
					type: 3,
					required: false,
					choices: MUSIC_STATIONS.map((k) => ({ name: RADIO_STATIONS[k].name, value: k })),
				},
			],
		});
	}

	public async run(client: Lavamusic, ctx: Context, args: string[]): Promise<any> {
		const key = (args[0] ?? "").toLowerCase() as RadioKey;
		if (key && MUSIC_STATIONS.includes(key)) {
			return await playRadio(client, ctx, key);
		}
		const list = MUSIC_STATIONS.map((k) => `\`${k}\` - ${RADIO_STATIONS[k].name}`).join("\n");
		return await ctx.sendMessage({
			embeds: [
				client
					.embed()
					.setColor(client.color.main)
					.setTitle("📻 Radyolar")
					.setDescription(`${list}\n\nÖrnek: \`!radyo paradise\``),
			],
		});
	}
}
