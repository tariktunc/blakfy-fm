import { Command, type Context, type Lavamusic } from "../../structures/index";
import { Connect, EmbedLinks, ReadMessageHistory, SendMessages, Speak, ViewChannel } from "../../utils/Permissions";
import { playRadio } from "../../utils/radio";

export default class TrtHaber extends Command {
	constructor(client: Lavamusic) {
		super(client, {
			name: "trthaber",
			description: {
				content: "TRT Haber Radyo canlı yayınını ses kanalında çalar",
				examples: ["trthaber"],
				usage: "trthaber",
			},
			category: "radio",
			aliases: ["trt"],
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
		return await playRadio(client, ctx, "trthaber");
	}
}
