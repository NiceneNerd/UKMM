import { Manifest } from "../Manifest/Manifest";

export class ModInfo extends Element {
  images = {};

  this(props) {
    if (props.mod && !(props.mod.hash in this.images)) {
      this.images[props.mod.hash] = Window.this.api("preview", props.mod.hash);
    }
    this.props = props;
  }

  render() {
    if (!this.props.mod) return <div></div>;
    const mod = this.props.mod.meta;
    return (
      <div styleset={__DIR__ + "ModInfo.css#ModInfo"}>
        {this.images[mod.hash] ? (
          <img class="preview" src={this.images[mod.hash]} />
        ) : (
          []
        )}
        <Row key="Name" val={mod.name} />
        <Row key="Version" val={mod.version.toPrecision(1)} />
        <Row key="Category" val={mod.category} />
        <Row key="Author" val={mod.author} />
        {mod.url ? <Row key="Webpage" val={mod.url} /> : []}
        <Long key="Description" markdown={true} val={mod.description} />
        {mod.option_groups?.length > 0 ? (
          <Long
            key="Options"
            val={
              <div class="hbox">
                {mod.option_groups.flatMap(group =>
                  group.options.map(opt => (
                    <div
                      class={
                        "pill " +
                        (!this.props.mod.enabled_options.includes(opt.name) &&
                          "disabled")
                      }>
                      {opt.name}
                    </div>
                  ))
                )}
              </div>
            }
          />
        ) : (
          []
        )}
        <Long
          key="Manifest"
          className="manifest"
          val={<Manifest manifest={this.props.mod.manifest} />}
        />
      </div>
    );
  }
}

const Row = ({ key, val }) => (
  <div class="row">
    <div class="label">{key}</div>
    <div class="data">{val}</div>
  </div>
);

const Long = ({ key, val, markdown, className }) => (
  <div class={"long " + (className ? className : "")}>
    <div class="label">{key}</div>
    <div class={"data " + (markdown && "md")}>{val}</div>
  </div>
);
