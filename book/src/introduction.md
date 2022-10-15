# Introduction

Once upon a time, there was a program called BCML, that is, Breath of the Wild
Cross-platform Mod Manager. It existed because Breath of the Wild is very rather
structurally hard to mod. The resource packing system, the resource size table,
and similar features of the game make mods very collision-prone. Crashes, bugs,
and other similar maladies appear frequently when multiple mods are naively
combined. BCML was born in this darkness to heal broken RSTB files, colliding
TitleBG packs, and similar woes.

Alas, BCML was ill-fated from the start. The idiosyncracies of Python, the
improvised and *ad hoc* nature of the solutions, and the growing complexity of
an expanding feature set wed to backward compatibility all conspired to ensure
it would one day grind to a halt.

Enter **UKMM**, U-King Mod Manager, a complete, ground-up replacement for
everything BCML did and more, written in pure Rust and compiled to a single
binary. UKMM incorporates all of the lessons, skills, and general experience
accumulated through the whole history of BOTW modding since BCML first began in
a smooth, reliable, and robust mod management solution which solves nearly
everything that made people (sometimes justifiably) rage and screech about
BCML.[^1]

[^1]: Except, of course, the requirement to have a complete game dump. There
will never be a way around that, guys.]