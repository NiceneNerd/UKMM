import Modal from "../Modal";

export const Error = ({ error }) => (
  <Modal>
    <p style="padding-top: 16dip; white-space: pre;" selectable>
      {error.backtrace || error}
    </p>
  </Modal>
);
