import { useState, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";

interface UserInfo {
  email: String;
  name: String;
  picture: string | null;
  is_admin: boolean;
}

interface Patient {
  NAME: string;
  MRN: string;
  LOCATION: string;
  GENDER: string;
  EMAIL: string;
  PHONE: string;
  DIAGNOSIS: string;
  NOTES: string;
}

interface DashboardProps {
  user: UserInfo;
  onSignOut: () => void;
  theme: 'dark' | 'light';
  onToggleTheme: () => void;
}

type StatusType = "success" | "warning" | "danger" | "info" | "loading";

export default function Dashboard({ user, onSignOut, theme, onToggleTheme }: DashboardProps) {
  // Database state
  const [patients, setPatients] = useState<Patient[]>([]);
  const [loading, setLoading] = useState(false);
  const [status, setStatus] = useState("Connecting to Database...");
  const [statusType, setStatusType] = useState<StatusType>("loading");

  // Search state
  const [searchName, setSearchName] = useState("");
  const [searchMrn, setSearchMrn] = useState("");
  const [searchDiag, setSearchDiag] = useState("");
  const [searchLoc, setSearchLoc] = useState("");

  // Form state
  const [formName, setFormName] = useState("");
  const [formMrn, setFormMrn] = useState("");
  const [formLoc, setFormLoc] = useState("");
  const [formGender, setFormGender] = useState("");
  const [formEmail, setFormEmail] = useState("");
  const [formPhone, setFormPhone] = useState("");
  const [formDiag, setFormDiag] = useState("");
  const [formNotes, setFormNotes] = useState("");

  // Table selection & sorting state
  const [selectedMrn, setSelectedMrn] = useState<string | null>(null);
  const [sortColumn, setSortColumn] = useState<keyof Patient | null>(null);
  const [sortAsc, setSortAsc] = useState(true);

  // Sync data from Google Sheet
  const loadData = async () => {
    setLoading(true);
    setStatus("Syncing with Database...");
    setStatusType("loading");
    try {
      const records = await invoke<Patient[]>("get_patients");
      setPatients(records);
      setStatus("Connected to Database");
      setStatusType("success");
    } catch (err) {
      console.error(err);
      setStatus(`Database Disconnected: ${err}`);
      setStatusType("danger");
    } finally {
      setLoading(false);
    }
  };

  // Run on mount
  useEffect(() => {
    loadData();
  }, []);

  // Form helpers
  const clearFields = () => {
    setFormName("");
    setFormMrn("");
    setFormLoc("");
    setFormGender("");
    setFormEmail("");
    setFormPhone("");
    setFormDiag("");
    setFormNotes("");
    setSelectedMrn(null);
    setStatus("Fields cleared.");
    setStatusType("info");
  };

  const handleRowSelect = (patient: Patient) => {
    setSelectedMrn(patient.MRN);
    setFormName(patient.NAME);
    setFormMrn(patient.MRN);
    setFormLoc(patient.LOCATION);
    setFormGender(patient.GENDER);
    setFormEmail(patient.EMAIL);
    setFormPhone(patient.PHONE);
    setFormDiag(patient.DIAGNOSIS);
    setFormNotes(patient.NOTES);
    setStatus(`Selected: ${patient.NAME} (${patient.MRN})`);
    setStatusType("info");
  };

  // In-memory search filter
  const filteredPatients = useMemo(() => {
    return patients.filter((p) => {
      const nameMatch = !searchName || p.NAME.toLowerCase().includes(searchName.toLowerCase());
      const mrnMatch = !searchMrn || p.MRN.toLowerCase().includes(searchMrn.toLowerCase());
      const diagMatch = !searchDiag || p.DIAGNOSIS.toLowerCase().includes(searchDiag.toLowerCase());
      const locMatch = !searchLoc || p.LOCATION.toLowerCase().includes(searchLoc.toLowerCase());
      return nameMatch && mrnMatch && diagMatch && locMatch;
    });
  }, [patients, searchName, searchMrn, searchDiag, searchLoc]);

  // Sorting
  const sortedPatients = useMemo(() => {
    if (!sortColumn) return filteredPatients;
    const sorted = [...filteredPatients];
    sorted.sort((a, b) => {
      let valA = a[sortColumn] || "";
      let valB = b[sortColumn] || "";

      // Numerical sort for MRN, PHONE, or numeric locations
      const isNumeric = sortColumn === "MRN" || sortColumn === "PHONE" || sortColumn === "LOCATION";
      if (isNumeric) {
        const numA = parseInt(valA.replace(/\D/g, "")) || 0;
        const numB = parseInt(valB.replace(/\D/g, "")) || 0;
        return sortAsc ? numA - numB : numB - numA;
      }

      // Alphabetical sort
      return sortAsc
        ? valA.toLowerCase().localeCompare(valB.toLowerCase())
        : valB.toLowerCase().localeCompare(valA.toLowerCase());
    });
    return sorted;
  }, [filteredPatients, sortColumn, sortAsc]);

  const handleSort = (column: keyof Patient) => {
    if (sortColumn === column) {
      setSortAsc(!sortAsc);
    } else {
      setSortColumn(column);
      setSortAsc(true);
    }
  };

  const isAdmin =
    user.is_admin &&
    String(user.email).trim().toLowerCase() === "stephen.dias@neuropedia.ae";

  // CRUD Operations
  const addPatient = async () => {
    if (!isAdmin) {
      setStatus("Read-Only Mode: Only stephen.dias@neuropedia.ae can add records.");
      setStatusType("warning");
      return;
    }
    if (!formName.trim() || !formMrn.trim()) {
      setStatus("Name and MRN are required fields.");
      setStatusType("warning");
      return;
    }

    // Check duplicate MRN
    if (patients.some((p) => p.MRN.trim() === formMrn.trim())) {
      setStatus(`Failed: MRN ${formMrn} already exists in database.`);
      setStatusType("danger");
      return;
    }

    setLoading(true);
    setStatus("Adding record...");
    setStatusType("loading");
    try {
      const patient: Patient = {
        NAME: formName.trim(),
        MRN: formMrn.trim(),
        LOCATION: formLoc.trim(),
        GENDER: formGender.trim(),
        EMAIL: formEmail.trim(),
        PHONE: formPhone.trim(),
        DIAGNOSIS: formDiag.trim(),
        NOTES: formNotes.trim(),
      };
      await invoke("add_patient", { patient });
      setStatus("Record added successfully.");
      setStatusType("success");
      await loadData();
      clearFields();
    } catch (err) {
      console.error(err);
      setStatus(`Failed to add record: ${err}`);
      setStatusType("danger");
    } finally {
      setLoading(false);
    }
  };

  const updatePatient = async () => {
    if (!isAdmin) {
      setStatus("Read-Only Mode: Only stephen.dias@neuropedia.ae can update records.");
      setStatusType("warning");
      return;
    }
    if (!formMrn.trim()) {
      setStatus("Please select a record or provide MRN to update.");
      setStatusType("warning");
      return;
    }

    const confirm = window.confirm("Are you sure you want to update this record?");
    if (!confirm) return;

    setLoading(true);
    setStatus("Updating record...");
    setStatusType("loading");
    try {
      const patient: Patient = {
        NAME: formName.trim(),
        MRN: formMrn.trim(),
        LOCATION: formLoc.trim(),
        GENDER: formGender.trim(),
        EMAIL: formEmail.trim(),
        PHONE: formPhone.trim(),
        DIAGNOSIS: formDiag.trim(),
        NOTES: formNotes.trim(),
      };
      await invoke("update_patient", { patient });
      setStatus("Record updated successfully.");
      setStatusType("success");
      await loadData();
    } catch (err) {
      console.error(err);
      setStatus(`Failed to update record: ${err}`);
      setStatusType("danger");
    } finally {
      setLoading(false);
    }
  };

  const deletePatient = async () => {
    if (!isAdmin) {
      setStatus("Read-Only Mode: Only stephen.dias@neuropedia.ae can delete records.");
      setStatusType("warning");
      return;
    }
    if (!formMrn.trim()) {
      setStatus("Please select a record to delete.");
      setStatusType("warning");
      return;
    }

    const confirm = window.confirm("Are you sure you want to delete this record permanently?");
    if (!confirm) return;

    setLoading(true);
    setStatus("Deleting record...");
    setStatusType("loading");
    try {
      await invoke("delete_patient", { mrn: formMrn.trim() });
      setStatus("Record deleted successfully.");
      setStatusType("success");
      await loadData();
      clearFields();
    } catch (err) {
      console.error(err);
      setStatus(`Failed to delete record: ${err}`);
      setStatusType("danger");
    } finally {
      setLoading(false);
    }
  };

  // Keyboard Shortcuts (Alt + key)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.altKey) {
        switch (e.key.toLowerCase()) {
          case "s":
            e.preventDefault();
            // Focus name search input
            document.getElementById("search-name")?.focus();
            setStatus("Search focused.");
            setStatusType("info");
            break;
          case "a":
            e.preventDefault();
            if (isAdmin) addPatient();
            break;
          case "u":
            e.preventDefault();
            if (isAdmin) updatePatient();
            break;
          case "d":
            e.preventDefault();
            if (isAdmin) deletePatient();
            break;
          case "c":
            e.preventDefault();
            clearFields();
            break;
          case "y":
            e.preventDefault();
            loadData();
            break;
          default:
            break;
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [formName, formMrn, formLoc, formGender, formEmail, formPhone, formDiag, formNotes, patients, isAdmin]);

  return (
    <div className="app-container">
      {/* Header Profile Bar */}
      <header className="header-bar">
        <div className="app-title-group">
          <h1>NeuroKidsDoc Patient Database</h1>
          <p>Diagnosis Database</p>
        </div>

        <div className="profile-group">
          {user.picture ? (
            <img src={user.picture} alt={String(user.name)} className="profile-avatar" />
          ) : (
            <div className="profile-avatar">
              {user.name ? String(user.name).charAt(0).toUpperCase() : "U"}
            </div>
          )}
          <div className="profile-details">
            <span className="profile-name">{String(user.name)}</span>
            <span className="profile-email">{String(user.email)}</span>
          </div>
          <button className="sign-out-btn" onClick={onToggleTheme} style={{ marginRight: "0.25rem" }}>
            {theme === 'dark' ? "☀️ Light" : "🌙 Dark"}
          </button>
          <button className="sign-out-btn" onClick={onSignOut}>
            Sign Out
          </button>
        </div>
      </header>

      {/* Main Content Area */}
      <main className="dashboard-content">
        {/* Left Side: Table & Search */}
        <div className="left-panel">
          {/* Search section */}
          <div className="search-bar">
            <div className="search-input-group">
              <input
                id="search-name"
                className="search-input"
                type="text"
                placeholder="Search Name..."
                value={searchName}
                onChange={(e) => setSearchName(e.target.value)}
              />
            </div>
            <div className="search-input-group">
              <input
                className="search-input"
                type="text"
                placeholder="Search MRN..."
                value={searchMrn}
                onChange={(e) => setSearchMrn(e.target.value)}
              />
            </div>
            <div className="search-input-group">
              <input
                className="search-input"
                type="text"
                placeholder="Search Diagnosis..."
                value={searchDiag}
                onChange={(e) => setSearchDiag(e.target.value)}
              />
            </div>
            <div className="search-input-group">
              <input
                className="search-input"
                type="text"
                placeholder="Search Location..."
                value={searchLoc}
                onChange={(e) => setSearchLoc(e.target.value)}
              />
            </div>
            <button className="search-btn" onClick={loadData}>
              Sync
            </button>
            <button 
              className="search-btn" 
              style={{ backgroundColor: "var(--bg-tertiary)", color: "var(--text-secondary)", border: "1px solid var(--border-color)" }} 
              onClick={() => { setSearchName(""); setSearchMrn(""); setSearchDiag(""); setSearchLoc(""); }}
            >
              Clear
            </button>
          </div>

          {/* Patient Table Card */}
          <div className="table-card">
            {loading && (
              <div className="loading-overlay">
                <div className="spinner"></div>
                <span>Communicating with Google Sheets...</span>
              </div>
            )}
            
            <div className="table-wrapper">
              <table className="patient-table">
                <thead>
                  <tr>
                    <th onClick={() => handleSort("NAME")}>
                      NAME {sortColumn === "NAME" && (sortAsc ? "▲" : "▼")}
                    </th>
                    <th onClick={() => handleSort("MRN")}>
                      MRN {sortColumn === "MRN" && (sortAsc ? "▲" : "▼")}
                    </th>
                    <th onClick={() => handleSort("LOCATION")}>
                      LOCATION {sortColumn === "LOCATION" && (sortAsc ? "▲" : "▼")}
                    </th>
                    <th onClick={() => handleSort("GENDER")}>
                      GENDER {sortColumn === "GENDER" && (sortAsc ? "▲" : "▼")}
                    </th>
                    <th onClick={() => handleSort("EMAIL")}>
                      EMAIL {sortColumn === "EMAIL" && (sortAsc ? "▲" : "▼")}
                    </th>
                    <th onClick={() => handleSort("PHONE")}>
                      PHONE {sortColumn === "PHONE" && (sortAsc ? "▲" : "▼")}
                    </th>
                    <th onClick={() => handleSort("DIAGNOSIS")}>
                      DIAGNOSIS {sortColumn === "DIAGNOSIS" && (sortAsc ? "▲" : "▼")}
                    </th>
                    <th onClick={() => handleSort("NOTES")}>
                      NOTES {sortColumn === "NOTES" && (sortAsc ? "▲" : "▼")}
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {sortedPatients.length > 0 ? (
                    sortedPatients.map((patient) => (
                      <tr
                        key={patient.MRN}
                        className={selectedMrn === patient.MRN ? "selected" : ""}
                        onClick={() => handleRowSelect(patient)}
                      >
                        <td>{patient.NAME}</td>
                        <td>{patient.MRN}</td>
                        <td>{patient.LOCATION}</td>
                        <td>{patient.GENDER}</td>
                        <td>{patient.EMAIL}</td>
                        <td>{patient.PHONE}</td>
                        <td>{patient.DIAGNOSIS}</td>
                        <td title={patient.NOTES}>{patient.NOTES}</td>
                      </tr>
                    ))
                  ) : (
                    <tr>
                      <td colSpan={8} className="no-records">
                        {loading ? "Loading data..." : "No patient records found."}
                      </td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>
          </div>
        </div>

        {/* Right Side: Form details */}
        <div className="form-panel">
          <h3 className="panel-header">Patient Record Details</h3>
          {!isAdmin && (
            <div className="login-status warning" style={{ fontSize: "0.8rem", padding: "0.5rem 0.75rem", marginBottom: "0.5rem", borderRadius: "6px", backgroundColor: "rgba(245, 158, 11, 0.15)", color: "var(--warning)", border: "1px solid rgba(245, 158, 11, 0.3)" }}>
              <strong>Read-Only Mode:</strong> Contact Stephen Dias for write access.
            </div>
          )}
          <div className="form-grid">
            <div className="form-group">
              <label>Name</label>
              <input type="text" value={formName} onChange={(e) => setFormName(e.target.value)} disabled={!isAdmin} />
            </div>
            <div className="form-group">
              <label>MRN (Unique Medical Record Number)</label>
              <input
                type="text"
                value={formMrn}
                onChange={(e) => setFormMrn(e.target.value)}
                disabled={selectedMrn !== null || !isAdmin} // MRN shouldn't be edited once record is selected
              />
            </div>
            <div className="form-group">
              <label>Location</label>
              <input type="text" value={formLoc} onChange={(e) => setFormLoc(e.target.value)} disabled={!isAdmin} />
            </div>
            <div className="form-group">
              <label>Gender</label>
              <select value={formGender} onChange={(e) => setFormGender(e.target.value)} disabled={!isAdmin}>
                <option value="">Select...</option>
                <option value="Male">Male</option>
                <option value="Female">Female</option>
                <option value="Other">Other</option>
              </select>
            </div>
            <div className="form-group">
              <label>Email</label>
              <input type="email" value={formEmail} onChange={(e) => setFormEmail(e.target.value)} disabled={!isAdmin} />
            </div>
            <div className="form-group">
              <label>Phone</label>
              <input type="tel" value={formPhone} onChange={(e) => setFormPhone(e.target.value)} disabled={!isAdmin} />
            </div>
            <div className="form-group">
              <label>Diagnosis</label>
              <input type="text" value={formDiag} onChange={(e) => setFormDiag(e.target.value)} disabled={!isAdmin} />
            </div>
            <div className="form-group">
              <label>Side Notes</label>
              <textarea value={formNotes} onChange={(e) => setFormNotes(e.target.value)} disabled={!isAdmin} />
            </div>
          </div>

          <div className="button-grid">
            <button
              className="action-btn add"
              onClick={addPatient}
              disabled={loading || !isAdmin}
              title={!isAdmin ? "Admin only (stephen.dias@neuropedia.ae)" : "Add new patient record"}
            >
              {!isAdmin && "🔒 "}Add New
            </button>
            <button
              className="action-btn update"
              onClick={updatePatient}
              disabled={loading || !isAdmin}
              title={!isAdmin ? "Admin only (stephen.dias@neuropedia.ae)" : "Update selected patient record"}
            >
              {!isAdmin && "🔒 "}Update
            </button>
            <button
              className="action-btn delete"
              onClick={deletePatient}
              disabled={loading || !isAdmin}
              title={!isAdmin ? "Admin only (stephen.dias@neuropedia.ae)" : "Delete selected patient record"}
            >
              {!isAdmin && "🔒 "}Delete
            </button>
            <button className="action-btn clear" onClick={clearFields} disabled={loading}>
              Clear
            </button>
            <button className="action-btn sync" onClick={loadData} disabled={loading}>
              Sync Now
            </button>
          </div>
        </div>
      </main>

      {/* Footer Status Bar */}
      <footer className="status-bar">
        <div className="status-indicator">
          <span
            className={`status-dot ${
              statusType === "success"
                ? "green"
                : statusType === "warning" || statusType === "loading"
                ? "yellow"
                : "red"
            }`}
          ></span>
          <span>{status}</span>
        </div>
        <span>Created by: Stephen Dias | Shortcuts: Alt+S (Search), Alt+A (Add), Alt+U (Update), Alt+D (Delete), Alt+C (Clear), Alt+Y (Sync)</span>
      </footer>
    </div>
  );
}
