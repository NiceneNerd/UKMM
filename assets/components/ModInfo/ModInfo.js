import { Info, Row, Long } from "../Info/Info";
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
      <Info>
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
                        (this.props.mod.enabled_options
                          .map(opt => opt.path)
                          .includes(opt.path)
                          ? ""
                          : "disabled")
                      }
                    >
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
      </Info>
    );
  }
}
