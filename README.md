<p align="center"><img src="docs/assets/icon.svg" width="72" alt="LinuxDrift"></p>
<h1 align="center">LinuxDrift</h1>
<p align="center"><strong>Drift / Deriva, ora su Linux.</strong><br>Un salvaschermo nativo. Dieci palette. Nessuna distrazione.</p>
<p align="center">
  <a href="https://spacecdr.github.io/LinuxDrift/"><strong>Visita il sito</strong></a> ·
  <a href="https://github.com/spacecdr/LinuxDrift/releases/latest"><strong>Scarica il .deb</strong></a> ·
  <a href="#installazione-deb">Installazione</a> ·
  <a href="https://github.com/spacecdr/LinuxDrift/issues">Segnala un problema</a>
</p>
<p align="center"><strong>X11 · Wayland · XScreenSaver · Offline · MIT</strong></p>

<p align="center"><a href="https://spacecdr.github.io/LinuxDrift/"><img src="docs/assets/original.webp" width="100%" alt="LinuxDrift in esecuzione: palette Original, linee multicolore su sfondo nero"></a></p>

**Versione Linux ispirata al salvaschermo Drift / Deriva di macOS**, pronta da installare come salvaschermo.
Basata su [Flux di Sander Melnikov](https://github.com/sandydoo/flux).
Non è un port ufficiale Apple. Mostra l'animazione senza slogan o controlli; dalla versione 1.1 è disponibile un orologio opzionale.
Funziona **interamente offline**: nessun browser, server web, account o download a runtime.
Le dieci palette originali, il font Inter e l’interfaccia di configurazione sono incorporati nel binario.

<table>
<tr><td><img src="docs/assets/poolside.webp" alt="Palette Poolside, linee azzurre"></td><td><img src="docs/assets/plasma.webp" alt="Palette Plasma, linee arancioni e dorate"></td></tr>
<tr><td align="center"><strong>Poolside</strong> · Toni freddi</td><td align="center"><strong>Plasma</strong> · Toni caldi</td></tr>
</table>

Le anteprime sono catture reali di LinuxDrift. [Guarda la pagina del progetto →](https://spacecdr.github.io/LinuxDrift/)


## Requisiti di sistema

- **Pacchetto verificato: Ubuntu 24.04 LTS, 64 bit x86_64 (amd64).**
- Sessione grafica **X11 oppure Wayland**; selezione automatica all'avvio.
- Driver **Vulkan** compatibile con wgpu; Mesa o driver proprietario della GPU. Il rendering
  software Mesa/llvmpipe funziona, ma per l'uso quotidiano è consigliata una GPU con accelerazione.
- **Python 3, PyGObject e GTK 4** per la finestra `--config`.
- **XScreenSaver** per comparire in `xscreensaver-demo` e attivarsi dopo inattività su X11.
  La modalità autonoma non richiede XScreenSaver.
- **glibc 2.39 o successiva** per questo binario. Le altre dipendenze sono dichiarate nel `.deb` e controllate da `apt`.

Altre distribuzioni Debian/Ubuntu possono funzionare se soddisfano le dipendenze, ma non sono
ancora verificate. Ubuntu 22.04, Debian 12 e le architetture ARM non sono destinazioni del primo
pacchetto: compilare dai sorgenti sul sistema interessato. Non richiede Internet durante l'uso.

## Installazione .deb

Scaricare il pacchetto e il relativo checksum dalla [pagina Releases](https://github.com/spacecdr/LinuxDrift/releases/latest).


Pacchetto per Debian/Ubuntu, compilato inizialmente su Ubuntu 24.04, architettura amd64:

```sh
sudo apt install ./linuxdrift_1.1.0_amd64.deb
linuxdrift
linuxdrift --config
```

Nel menu applicazioni sono disponibili **LinuxDrift** e **Impostazioni LinuxDrift**.
Il programma funziona offline dopo l'installazione; su una macchina senza le dipendenze,
`apt` deve procurarle durante l'installazione (oppure vanno fornite offline insieme al pacchetto).
Occorre un driver Vulkan compatibile. Mesa supporta anche il rendering software, più lento.
`--config` usa Python 3, PyGObject e GTK 4, dichiarati nelle dipendenze del pacchetto.

## Utilizzo

- `linuxdrift`: animazione a schermo intero, cursore nascosto. Muovere il mouse, premere un tasto o fare clic per uscire; una breve tolleranza iniziale evita chiusure involontarie.
- `linuxdrift --config`: modifica palette, immagine personale, dimensioni delle linee, sfumatura, varianza, griglia, scala, fluido, rumore, seed e modalità diagnostiche. **Anteprima** prova le modifiche senza salvarle; **Salva** le conserva; **Annulla** lascia il file invariato.
- `linuxdrift --preview`: anteprima ridimensionabile con le impostazioni salvate; Esc chiude.
- `linuxdrift --check-config`: verifica impostazioni e immagine, senza una finestra grafica.
- `linuxdrift --print-config`: stampa la configurazione effettiva.
- `linuxdrift --backend x11` / `--backend wayland`: forza il backend per diagnostica.
- `linuxdrift --duration 10`: termina dopo dieci secondi di rendering.

Le impostazioni vengono lette a ogni avvio da `$XDG_CONFIG_HOME/linuxdrift/config.json`,
o da `~/.config/linuxdrift/config.json` quando XDG_CONFIG_HOME non è assoluto o non è impostato.
In assenza del file si usano i valori originali. I salvataggi sono atomici, con permessi utente;
JSON non valido, opzioni sconosciute e valori fuori intervallo vengono segnalati senza sovrascrivere il file.
Le immagini personali devono avere un percorso assoluto e restare disponibili sul disco.

## Orologio e preset prestazioni (1.1)

![Orologio opzionale con sfondo sfumato, cattura reale](docs/assets/clock.webp)

In `linuxdrift --config` puoi attivare **Mostra orologio HH:MM**, scegliere la dimensione
(4–25% del lato corto dello schermo), sette posizioni fisse oppure **Casuale**.
In modalità casuale cambia posizione ogni 5–3600 secondi (predefinito: 60), rimanendo
all’interno dello schermo, anche dopo un ridimensionamento. L’ora è quella locale del computer,
in formato 24 ore. L’orologio è disattivato per impostazione iniziale.

Lo sfondo nero è opzionale: **opacità 0** lo disattiva; aumentandola migliora il contrasto.
La sfumatura verso il trasparente ha cinque livelli: Netto, Leggera, Media, Morbida e Diffusa.
Il carattere incluso è **Inter Light**, un’alternativa libera dall’aspetto vicino a macOS,
non il font Apple. [Inter](https://github.com/rsms/inter) è distribuito con la propria
[licenza SIL OFL](linuxdrift/assets/Inter-LICENSE.txt); i font Apple non sono inclusi.

**Salva** conserva orologio e animazione nello stesso `config.json`. Le configurazioni precedenti
restano valide e mantengono i loro valori. Il testo viene rasterizzato solo quando cambia il minuto
o la risoluzione; il disegno aggiunge un solo rettangolo GPU per fotogramma, senza finestre separate.

I dieci preset agiscono su risoluzione del fluido, iterazioni, densità delle linee e frequenza
della simulazione. Conservano palette, orologio e personalizzazioni estetiche. Il carico è una
**stima relativa**, non un benchmark: risoluzione, driver e hardware cambiano il risultato.
Il lavoro è prevalentemente GPU; con rendering software ricade sulla CPU.

| Preset | Carico GPU previsto | Fluido | Pressione | Griglia | Passi/s |
|---|---|---:|---:|---:|---:|
| Eco | minimo | 32 | 4 | 32 | 30 |
| Leggero | molto basso | 48 | 6 | 28 | 45 |
| Haswell fluido | basso | 64 | 8 | 24 | 60 |
| Leggero Plus | basso–medio | 64 | 12 | 24 | 60 |
| Bilanciato | medio | 80 | 10 | 22 | 60 |
| Bilanciato Plus | medio–alto | 96 | 12 | 20 | 60 |
| Dettagliato | alto | 112 | 16 | 18 | 60 |
| Originale Flux | alto | 128 | 20 | 16 | 60 |
| Alta qualità | molto alto | 160 | 24 | 14 | 60 |
| Ultra | massimo | 192 | 30 | 12 | 60 |

**Haswell fluido** riprende il profilo 64 / 8 / 24 risultato fluido nella prova dell’utente;
non garantisce le stesse prestazioni su tutte le GPU Haswell. Se modifichi i parametri del preset,
il menu torna a **Personalizzato**. Selezionare un preset nella finestra non scrive nulla fino a **Salva**.

Prove temporanee da terminale, senza modificare il file:

```sh
linuxdrift --preset haswell --clock
linuxdrift --no-clock
linuxdrift --list-presets
linuxdrift --set 'clock={"enabled":true,"position":"random","moveInterval":60,"size":12,"backgroundOpacity":0.6,"backgroundFeather":75}'
```

`xscreensaver-demo` offre anche la scelta del preset e Mostra/Nascondi orologio;
le impostazioni dettagliate restano accessibili con `linuxdrift --config`.

## X11 e Wayland

Il programma usa finestre native tramite winit: seleziona Wayland nelle sessioni Wayland
anche quando è disponibile XWayland; seleziona X11 nelle sessioni X11. In assenza di un tipo
sessione dichiarato dà precedenza a WAYLAND_DISPLAY, poi a DISPLAY. Il rendering usa wgpu/Vulkan.

Questa versione apre una finestra fullscreen su un monitor scelto dal compositor/window manager.
Non è un blocco schermo e non implementa il protocollo session-lock. L'avvio automatico dopo
inattività va configurato nel proprio desktop o idle manager: non esiste un protocollo idle
universale supportato da tutti i compositor Wayland. Per esempio, con `swayidle` già installato:

```sh
swayidle -w timeout 300 'linuxdrift'
```

### Avvio dopo inattività su GNOME

Lo script [scripts/linuxdrift-idle](scripts/linuxdrift-idle) avvia LinuxDrift dopo
un intervallo di inattività su GNOME, anche in una sessione Wayland. Richiede
LinuxDrift installato in `/usr/bin/linuxdrift`, Python 3 e PyGObject (`python3-gi`),
e usa i servizi D-Bus della sessione GNOME/Mutter. Non è un gestore idle universale
per tutti i desktop Wayland.

Dalla cartella del repository:

```sh
install -Dm755 scripts/linuxdrift-idle ~/.local/bin/linuxdrift-idle
~/.local/bin/linuxdrift-idle --check
~/.local/bin/linuxdrift-idle --minutes 5
```

`--check` verifica i servizi senza avviare il salvaschermo. L’intervallo predefinito
è cinque minuti. Lo script rispetta il blocco schermo e le richieste GNOME di
inibizione dell’inattività; al ritorno dell’attività chiude il salvaschermo che ha
avviato. Non sostituisce il blocco schermo e non modifica le impostazioni del desktop.
Ctrl+C ferma il monitor e il suo salvaschermo; può essere attiva una sola istanza.

Per avviarlo al login, aggiungi nelle **Applicazioni d’avvio** di GNOME il comando
con il percorso assoluto della tua home, per esempio
`/home/NOMEUTENTE/.local/bin/linuxdrift-idle --minutes 5`.
Lo script è fornito nel repository; non viene installato o attivato automaticamente dal `.deb`.

### Integrazione XScreenSaver

Il `.deb` installa il modulo e la scheda di configurazione XML riconosciuta da
`xscreensaver-demo` / `xscreensaver-settings`. L'installazione tramite `sudo apt`
registra LinuxDrift nella lista dell'utente che installa; per gli altri utenti la
registrazione avviene al login. Per registrarlo subito nella sessione corrente:

```sh
linuxdrift-register-xscreensaver
xscreensaver-demo
```

Cercare **LinuxDrift**, selezionarlo e aprire **Impostazioni**.
L'anteprima viene disegnata nella finestra del gestore; il daemon gestisce ogni
monitor, l'inattività e l'eventuale blocco. LinuxDrift accetta `--root`, `-root`,
`--window-id`, `-window-id` e la variabile `XSCREENSAVER_WINDOW`.
L'incorporamento forza X11 anche se il gestore viene aperto tramite XWayland.
XScreenSaver gestisce il blocco su X11; su Wayland usare il gestore nativo del desktop.

La registrazione aggiunge una sola voce in `~/.xscreensaver`, mantiene l'ordine e
la selezione esistenti e crea una copia `.xscreensaver.before-linuxdrift`. Non attiva
né sostituisce il daemon e non cambia le preferenze di blocco. Dopo la disinstallazione
la voce può essere rimossa dall'elenco del gestore (i file personali vengono conservati).

Le opzioni di XScreenSaver sono salvate da quel gestore in `~/.xscreensaver` e prevalgono
sulle opzioni equivalenti del file LinuxDrift. **Impostazioni LinuxDrift** / `--config`
offre tutte le opzioni, comprese immagini personali e canali rumore. Sono disponibili
anche gli override `--palette plasma` e `--set lineLength=300` per singolo avvio.

## Compilazione e pacchetto

Installare una versione Rust stable compatibile con il lockfile (verificata con Rust 1.99) e:

```sh
sudo apt install build-essential pkg-config libwayland-dev libxkbcommon-dev \
  libx11-dev libxrandr-dev libxi-dev libxcursor-dev libxkbcommon-x11-0 dpkg-dev \
  python3-gi gir1.2-gtk-4.0 libvulkan1 mesa-vulkan-drivers
cargo build --locked --release -p linuxdrift
cargo test --locked -p linuxdrift -p flux
./scripts/build-deb.sh --no-build
```

Il pacchetto viene prodotto in `dist/` con checksum SHA-256; le versioni minime delle librerie
sono ricavate dal sistema di compilazione. Per distribuzioni precedenti, compilare su quella
distribuzione invece di forzare l'installazione di un pacchetto con dipendenze incompatibili.
La prima compilazione scarica i crate Rust; l'applicazione installata non usa la rete.

## Origine e licenza

Basato sulla cronologia di sandydoo/flux, revisione `8836d0a` (Flux 7.2.1).
LinuxDrift è un fork pubblico di sandydoo/flux e conserva la cronologia upstream.
Lo sviluppo specifico Linux e i pacchetti installabili si trovano nel repository
[spacecdr/LinuxDrift](https://github.com/spacecdr/LinuxDrift).

Licenza MIT; copyright originale di Sander Melnikov conservato in [LICENSE](LICENSE).
I componenti web, WebAssembly e le vecchie applicazioni desktop upstream restano nei sorgenti
per tracciabilità, ma non sono inclusi né eseguiti nel pacchetto LinuxDrift.
