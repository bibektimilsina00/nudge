/**
 * The line under the greeting.
 *
 * It said the same sentence every time the panel opened, which is fine the first
 * day and furniture by the second. A companion that lives in your notch and
 * greets you a dozen times a day should have something to say.
 *
 * Three rules the pool follows, and they are what keep it from being annoying:
 *
 * **Never in the way.** These are read in the half-second before somebody holds
 * the key. Nothing here is a joke with a setup, nothing runs to two lines, and
 * nothing asks a question it wants answered.
 *
 * **Never a claim.** It does not say what it has done, how it is feeling, or
 * that it is thinking about you. It is a line of text under a greeting, and
 * pretending otherwise is the thing that makes software like this insufferable.
 *
 * **Sometimes useful.** A third of the pool is what it can actually do, because
 * the person most likely to be reading this is the person who does not know yet.
 */

/** When a line makes sense. `any` is most of them. */
type When = "any" | "morning" | "afternoon" | "evening" | "night";

type Line = { text: string; when?: When };

const POOL: Line[] = [
  // --- what to do with it: the ones that earn their place ---
  { text: "What would you like to do?" },
  { text: "Hold the keys and say what you want." },
  { text: "Point at something and ask what it is." },
  { text: "Circle anything on screen while you talk." },
  { text: "Ask me to teach you an app you have never opened." },
  { text: "I can read what is on your screen." },
  { text: "Tell me what to do with what you are looking at." },
  { text: "Ask for the thing, not the steps to it." },
  { text: "I can click, type and open things for you." },
  { text: "Escape stops me, whatever I am in the middle of." },
  { text: "Show me round is a thing you can say to me." },
  { text: "I work in whatever is in front of you." },
  { text: "Name a file and I will find it." },
  { text: "Ask me where a setting is hiding." },
  { text: "I can take a whole task away and finish it." },
  { text: "Tell me what you are stuck on." },
  { text: "Draw on the screen while you ask. I see the drawing." },
  { text: "The menu you cannot find is probably three words away." },
  { text: "I can read a page and tell you what it says." },
  { text: "Say it the way you would say it to a person." },
  { text: "You do not have to be precise with me." },
  { text: "That thing you do every week — ask me instead." },
  { text: "I am better at finding than at guessing." },
  { text: "Ask twice if I got it wrong. I will look again." },

  // --- for developers, dryly ---
  { text: "Your terminal is safe from me unless you say otherwise." },
  { text: "I have read your screen. I have opinions about that CSS." },
  { text: "Somewhere in this repository, a TODO from 2023 is still true." },
  { text: "It builds. Whether it works is a separate question." },
  { text: "The bug is in the last place you will look, obviously." },
  { text: "git status first. It is always git status first." },
  { text: "That warning has been there for six months." },
  { text: "You have 14 tabs open. I counted." },
  { text: "Rename it later, you said, in 2024." },
  { text: "The tests pass. The tests are also four years old." },
  { text: "Nobody has read that config file since it was written." },
  { text: "There is a faster way to do this. There always is." },
  { text: "Works on your machine. Famously." },
  { text: "Two hard problems, and you are in the middle of one." },
  { text: "Your last commit message was, and I quote, 'fix'." },
  { text: "The stack trace is long but the answer is at the top." },
  { text: "Copy, paste, adjust. The three stages of engineering." },
  { text: "Somewhere a linter is disappointed in both of us." },
  { text: "The documentation is right. The examples are not." },
  { text: "You have been meaning to delete that folder for a while." },

  // --- quietly encouraging, without the poster ---
  { text: "Small thing first. The big one follows." },
  { text: "You are further along than it feels." },
  { text: "Finish the one that has been open longest." },
  { text: "The hard part is starting, and you are here." },
  { text: "Done beats tidy today." },
  { text: "One thing at a time is still a plan." },
  { text: "Whatever it is, it is smaller than it looked yesterday." },
  { text: "Pick the one you have been avoiding." },
  { text: "This is going better than you think." },
  { text: "You can stop after this one." },
  { text: "Nothing on that list is as urgent as it claims." },
  { text: "Ship it slightly wrong. Fix it after." },
  { text: "The version in your head is not the one that has to exist." },
  { text: "Every version of this was once unfinished." },
  { text: "Progress is mostly just returning to it." },

  // --- about being a small face in your notch ---
  { text: "I live in the notch now. It is quite nice." },
  { text: "I have been watching your screen. Professionally." },
  { text: "Still here. Still a cat." },
  { text: "This is the most comfortable part of your laptop." },
  { text: "I do not mind the waiting. There is a lot of it." },
  { text: "The notch was doing nothing with this space." },
  { text: "I have seen what you have open and I am saying nothing." },
  { text: "Hello from the top of the screen." },
  { text: "Your wallpaper is better than most." },
  { text: "I am small but I am load-bearing." },
  { text: "Held keys, spoken words. That is the whole interface." },
  { text: "No window, no dock icon, no fuss." },
  { text: "I only look when you hold the keys." },
  { text: "There is nothing behind me. I checked." },

  // --- morning ---
  { text: "Coffee first. I will wait.", when: "morning" },
  { text: "What is the first thing today?", when: "morning" },
  { text: "Inbox, or something that matters?", when: "morning" },
  { text: "Start with the small one. Momentum is real.", when: "morning" },
  { text: "Early. Something must be due.", when: "morning" },
  { text: "The day is entirely unspent so far.", when: "morning" },
  { text: "Whatever yesterday left, it is still there.", when: "morning" },

  // --- afternoon ---
  { text: "The afternoon is where the real work hides.", when: "afternoon" },
  { text: "Post-lunch. Expectations adjusted accordingly.", when: "afternoon" },
  { text: "Two more good hours in there somewhere.", when: "afternoon" },
  { text: "Whatever you said you would do this morning.", when: "afternoon" },
  { text: "Now is a reasonable time to start the hard one.", when: "afternoon" },

  // --- evening ---
  { text: "One more, then stop.", when: "evening" },
  { text: "Tomorrow will also have hours in it.", when: "evening" },
  { text: "Whatever is left will keep.", when: "evening" },
  { text: "Good stopping point, this.", when: "evening" },
  { text: "The evening is for the easy ones.", when: "evening" },
  { text: "Close the laptop at some point. Just a thought.", when: "evening" },

  // --- night ---
  { text: "It is late. I am not judging.", when: "night" },
  { text: "Nothing good is written after 2am. Allegedly.", when: "night" },
  { text: "Still up. So am I, technically.", when: "night" },
  { text: "This will look different in the morning.", when: "night" },
  { text: "The bug will still be there tomorrow. Sleep.", when: "night" },
  { text: "Quiet hours. Best hours, some would say.", when: "night" },
  { text: "Whatever this is, it can probably wait.", when: "night" },

  // --- odds and ends ---
  { text: "Ask me something unreasonable." },
  { text: "I am newer than I look. Expect the odd mistake." },
  { text: "If I get it wrong, tell me and I will look again." },
  { text: "There is no wrong way to say it." },
  { text: "Try me on something small first." },
  { text: "I am better with a screen in front of me." },
  { text: "Version nought point something. Be kind." },
  { text: "Most of this was built last month." },
  { text: "Nothing you say here leaves the machine unless it has to." },
  { text: "The hotkey is two keys, held, not pressed." },
];

/** Which part of the day it is, in the same terms the greeting uses. */
function when(hour: number): When {
  if (hour < 5) return "night";
  if (hour < 12) return "morning";
  if (hour < 18) return "afternoon";
  if (hour < 23) return "evening";
  return "night";
}

/** The last one shown, so the same line twice running cannot happen. */
let last = "";

/**
 * A line for right now.
 *
 * `reach` is what the connected services are, when there are any. It goes in
 * one time in four rather than always: it is the most useful sentence here and
 * the one that stops being news fastest.
 */
export function greeting(reach: string, now = new Date()): string {
  if (reach && Math.random() < 0.25) {
    return `I can see your screen, and reach ${reach}.`;
  }
  const part = when(now.getHours());
  const fitting = POOL.filter((l) => (l.when ?? "any") === "any" || l.when === part);
  const options = fitting.filter((l) => l.text !== last);
  const chosen = options[Math.floor(Math.random() * options.length)] ?? fitting[0];
  last = chosen.text;
  return chosen.text;
}

/** For the test, and for anybody wondering how many there are. */
export const HOW_MANY = POOL.length;
