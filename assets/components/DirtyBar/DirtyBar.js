export const DirtyBar = ({ onApply }) => (
  <div styleset={__DIR__ + "DirtyBar.css#DirtyBar"}>
    <div class="label">Changes Pending Apply</div>
    <button class="icon" title="Apply Pending Changes" onClick={onApply}>
      Apply
    </button>
  </div>
);
