import { Command, type Context, type Lavamusic } from "../../structures/index";
import { Connect, EmbedLinks, ReadMessageHistory, SendMessages, Speak, ViewChannel } from "../../utils/Permissions";
import { playRadio } from "../../utils/radio";

export default class CnnTurk extends Command {
	constructor(client: Lavamusic) {
		super(client, {
			name: "cnnturk",
			description: {
				content: "CNN Türk Radyo canlı yayınını ses kanalında çalar",
				examples: ["cnnturk"],
				usage: "cnnturk",
			},
			category: "radio",
			aliases: ["cnn"],
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
		return await playRadio(client, ctx, "cnnturk");
	}
}
