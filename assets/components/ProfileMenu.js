export class ProfileMenu extends Element {
    this(props) {
        this.props = props;
    }

    render() {
        return (
            <div class="hbox" style="vertical-align: middle;">
              <div style="line-height: *;">Profile:</div>
              <select value={this.props.currentProfile}>
                {this.props.profiles.map((profile) => (
                  <option key={profile}>{profile}</option>
                ))}
              </select>
            </div>
        );
    }
}