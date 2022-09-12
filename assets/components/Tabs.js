export class Tabs extends Element {
    this(props, kids) {
        this.props = props;
        this.kids = kids;
        this.activeIndex = props.defaultIndex || 0;
        this.handleTabClick = this.handleTabClick.bind(this);
        this.buttonClass = this.buttonClass.bind(this);
    }

    handleTabClick(i) {
        this.componentUpdate({ activeIndex: i });
    }

    buttonClass(i) {
        return this.activeIndex == i
            ? "active tab-button"
            : "tab-button";
    }

    render() {
        return (
            <div styleset={__DIR__ + "Tabs.css#Tabs"}>
                <div class="strip">
                    {this.kids.map((kid, i) => {
                        return <div class={this.buttonClass(i)} onClick={() => this.handleTabClick(i)}>{kid[1]["label"]}</div>;
                    })}
                    <div style="width: *;">{" "}</div>
                </div>
                {this.kids[this.activeIndex]}
            </div>
        );
    }
}

export const Tab = (props, kids) => <div class="tab">{kids}</div>;
