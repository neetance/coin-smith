function PsbtViewer({ psbt }) {

    const copy = () => {
        navigator.clipboard.writeText(psbt);
    };

    return (
        <div className="card">

            <h3>PSBT (Base64)</h3>

            <textarea rows={6} value={psbt} readOnly />

            <br />

            <button onClick={copy}>Copy</button>

        </div>
    );
}

export default PsbtViewer;