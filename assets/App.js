import { ModList } from "./components/ModList";

export class App extends Element {
    constructor(props) {
        super(props);
        this.props = props;
        this.api = Window.this.xcall("GetApi");
        this.mods = [];
        this.handleToggle = this.handleToggle.bind(this);
    }

    componentDidMount() {
        this.componentUpdate({ mods: this.api.mods() });
    }

    handleToggle(mod) {
        console.log(`Toggling ${mod.meta.name}`);
        // let mod = this.mods.find(m => m == mod);
        mod.enabled = !mod.enabled;
        this.componentUpdate({ mods: this.mods });
    }

    render() {
        return (
            <div>
                <p>Hello world</p>
                <ModList mods={this.mods} onToggle={this.handleToggle} />
                <button>Testing a button</button>
            </div>
        );
    }
}