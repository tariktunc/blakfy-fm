import { Command, type Context, type Lavamusic } from "../../structures/index";
import { Connect, EmbedLinks, ReadMessageHistory, SendMessages, Speak, ViewChannel } from "../../utils/Permissions";
import { playRadio } from "../../utils/radio";

export default class Haberturk extends Command {
	constructor(client: Lavamusic) {
		super(client, {
			name: "haberturk",
			description: {
				content: "Habertürk canlı yayınının sesini ses kanalında çalar",
				examples: ["haberturk"],
				usage: "haberturk",
			},
			category: "radio",
			aliases: ["ht"],
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
			options: [],
		});
	}

	public async run(client: Lavamusic, ctx: Context): Promise<any> {
		return await playRadio(client, ctx, "haberturk");
	}
}
