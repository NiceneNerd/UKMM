import { ModList } from "./components/ModList";

export class App extends Element {
    constructor(props, kids) {
        super(props);
        this.props = props;
        this.api = Window.this.xcall("GetApi");
    }

    handleClick() {
        let mods = this.api.mods();
        this.api.check_hash(mods[0].hash);
    }

    render(props, kids) {
        return (
            <div>
                <p>Hello world</p>
                <ModList />
                <button onClick={() => this.handleClick()}>Testing a button</button>
            </div>
        );
    }
}