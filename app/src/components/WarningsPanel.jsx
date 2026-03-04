function WarningsPanel({ warnings }) {

    if (!warnings || warnings.length === 0) return null;

    return (
        <div className="card">

            <h3>Warnings</h3>

            {warnings.map((w, idx) => (
                <div key={idx} className="warning">
                    ⚠ {w.code}
                </div>
            ))}

        </div>
    );
}

export default WarningsPanel;