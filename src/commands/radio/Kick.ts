import { Command, type Context, type Lavamusic } from "../../structures/index";
import { Connect, EmbedLinks, ReadMessageHistory, SendMessages, Speak, ViewChannel } from "../../utils/Permissions";
import { playKick } from "../../utils/radio";

export default class Kick extends Command {
	constructor(client: Lavamusic) {
		super(client, {
			name: "kick",
			description: {
				content: "Yayında olan bir Kick kanalının sesini ses kanalında çalar",
				examples: ["kick kanaladi", "kick https://kick.com/kanaladi"],
				usage: "kick <kanal>",
			},
			category: "radio",
			aliases: [],
			cooldown: 3,
			args: true,
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
					name: "kanal",
					description: "Kick kanal adı veya linki",
					type: 3,
					required: true,
				},
			],
		});
	}

	public async run(client: Lavamusic, ctx: Context, args: string[]): Promise<any> {
		return await playKick(client, ctx, args.join(" "));
	}
}
