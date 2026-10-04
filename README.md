# LinuxDrift

**Versione Linux ispirata al salvaschermo Drift / Deriva di macOS**, pronta da installare come salvaschermo.
Basata su [Flux di Sander Melnikov](https://github.com/sandydoo/flux).
Non è un port ufficiale Apple. Mostra solo l'animazione, senza slogan, controlli o informazioni sovrapposte.
Funziona **interamente offline**: nessun browser, server web, account o download a runtime.
Le dieci palette originali e l'interfaccia di configurazione sono incorporate nel binario.

![LinuxDrift in esecuzione, palette Plasma](docs/linuxdrift.png)

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
sudo apt install ./linuxdrift_1.0.0_amd64.deb
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
  libx11-dev libxrandr-dev libxi-dev libxcursor-dev dpkg-dev \
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
