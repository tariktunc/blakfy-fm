import { Command, type Context, type Lavamusic } from "../../structures/index";
import { Connect, EmbedLinks, ReadMessageHistory, SendMessages, Speak, ViewChannel } from "../../utils/Permissions";
import { playRadio } from "../../utils/radio";

export default class KafaRadyo extends Command {
	constructor(client: Lavamusic) {
		super(client, {
			name: "kafaradyo",
			description: {
				content: "Kafa Radyo canlı yayınını ses kanalında çalar",
				examples: ["kafaradyo"],
				usage: "kafaradyo",
			},
			category: "radio",
			aliases: ["kafa"],
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
		return await playRadio(client, ctx, "kafaradyo");
	}
}
