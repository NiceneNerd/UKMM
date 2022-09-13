export class ProfileMenu extends Element {
  this(props) {
    this.props = props;
  }

  render() {
    return (
      <div styleset={__DIR__ + "ProfileMenu.css#ProfileMenu"}>
        <div style="height: *; vertical-align: middle;">Profile:</div>
        <select value={this.props.currentProfile}>
          {this.props.profiles.map((profile) => (
            <option key={profile}>{profile}</option>
          ))}
        </select>
      </div>
    );
  }
}
