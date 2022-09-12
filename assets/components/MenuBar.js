export class MenuBar extends Element {
    app;

    this(props,kids) {
        this.app = props.app;
    }

    render() {
        return <ul styleset={__DIR__ + "MenuBar.css#menu-bar"}>
            <li>File
                <menu>
                  <li.command name="new-file" accesskey="^N">New file <span class="accesskey">Ctrl+N</span></li>
                  <li.command name="open-file">Open file …</li>
                  <li.command name="save-file">Save file</li>
                  <li.command name="save-file-as">Save file as …</li>
                </menu>
            </li>
            <li>Edit
                <menu>
                  <li.command name="edit-copy">Copy</li>
                  <li.command name="edit-paste">Cut</li>
                  <li.command name="edit-paste">Paste</li>
                </menu>
            </li>
        </ul>;
    }

}
