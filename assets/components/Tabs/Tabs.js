export class Tabs extends Element {
  constructor(props, kids) {
    super(props);
    this.activeIndex = props.defaultIndex || 0;
    this.handleTabClick = this.handleTabClick.bind(this);
    this.buttonClass = this.buttonClass.bind(this);
  }

  this(props, kids) {
    this.props = props;
    this.kids = kids;
  }

  handleTabClick(i) {
    this.componentUpdate({ activeIndex: i });
  }

  buttonClass(i) {
    return this.activeIndex == i ? "active tab-button" : "tab-button";
  }

  render() {
    return (
      <div styleset={__DIR__ + "Tabs.css#Tabs"}>
        <div class="strip">
          {this.kids.map((kid, i) => {
            return (
              <div
                class={this.buttonClass(i)}
                onClick={() => this.handleTabClick(i)}>
                {kid[1]["label"]}
              </div>
            );
          })}
          <div style="width: *;"> </div>
        </div>
        {this.kids.map((kid, i) =>
          i == this.activeIndex
            ? JSX(kid[0], { ...kid[1], active: true }, kid[2])
            : kid
        )}
      </div>
    );
  }
}

export const Tab = ({ active }, kids) => (
  <div class={"tab " + (active ? "active" : "")}>{kids}</div>
);
