import {
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
  Grid,
  TextField,
  useMediaQuery,
  useTheme,
} from '@material-ui/core';
import Alert from '@material-ui/lab/Alert';
import PropTypes from 'prop-types';
import React, { useState } from 'react';
import { connect } from 'react-redux';
import DeleteIcon from '@material-ui/icons/Delete';

import {
  deleteBalance as deleteBalanceAction,
  createBalance as createBalanceAction,
  updateBalance as updateBalanceAction,
} from '../actions/balances';

const BalanceDialog = ({
  initialBalance,
  open,
  onClose,
  deleteBalance,
  createBalance,
  updateBalance,
}) => {
  const toIBAN = s => {
    const iban = s.trim().toUpperCase();
    if (iban === '' || /^[A-Z]{2}[0-9]{2}[0-9A-Z]{12,30}$/.test(iban))
      return iban;

    return null;
  };

  const toBalance = t => {
    const iban = toIBAN(t.iban);
    if (t.name.trim() === '' || iban === null) return null;

    const ret = {
      userId: -1,
      id: t.id,
      name: t.name.trim(),
      iban: iban !== '' ? iban : null,
    };

    return ret;
  };

  const toEditable = t => {
    return { ...t };
  };

  const theme = useTheme();
  const mdUp = useMediaQuery(theme.breakpoints.up('md'));

  const [balance, setBalance] = useState(toEditable(initialBalance));
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [triedToSave, setTriedToSave] = useState(false);

  const reset = () => {
    setBalance(toEditable(initialBalance));
    setTriedToSave(false);
  };

  const onDelete = () => {
    setConfirmOpen(false);
    deleteBalance(balance.id);
    onClose();
  };

  const onSave = () => {
    const t = toBalance(balance);

    if (t === null) {
      setTriedToSave(true);
      return;
    }

    if (t.id === null) {
      createBalance(t);
    } else {
      updateBalance(t);
    }

    onClose();
  };

  return (
    <>
      <Dialog
        fullScreen={!mdUp}
        open={open}
        disableBackdropClick
        onClose={() => {
          reset();
          onClose();
        }}
        onEntered={reset}
        onEnter={reset}
      >
        <DialogTitle id="form-dialog-title">
          {balance.id !== null ? 'Depot bearbeiten' : 'Depot erstellen'}
        </DialogTitle>
        <DialogContent>
          <Grid container spacing={1}>
            <Grid item xs={12}>
              <TextField
                margin="dense"
                label="Name"
                fullWidth
                value={balance.name}
                onChange={v =>
                  setBalance({
                    ...balance,
                    name: v.target.value,
                  })
                }
                error={balance.name.trim() === '' && triedToSave}
              />
            </Grid>
            <Grid item xs={12}>
              <TextField
                margin="dense"
                label="IBAN"
                fullWidth
                value={balance.iban}
                onChange={v =>
                  setBalance({
                    ...balance,
                    iban: v.target.value,
                  })
                }
                error={toIBAN(balance.iban) === null && triedToSave}
              />
            </Grid>
          </Grid>
          {toBalance(balance) === null && triedToSave && (
            <Alert severity="error">Depot ist unvollständig!</Alert>
          )}
        </DialogContent>
        <DialogActions>
          {balance.id !== null && (
            <Button
              variant="outlined"
              color="secondary"
              startIcon={<DeleteIcon />}
              onClick={() => setConfirmOpen(true)}
            >
              Löschen
            </Button>
          )}
          <Button color="primary" onClick={onClose}>
            Abbrechen
          </Button>
          <Button color="primary" onClick={onSave}>
            Speichern
          </Button>
        </DialogActions>
      </Dialog>
      <Dialog
        open={confirmOpen}
        onClose={() => setConfirmOpen(false)}
        aria-labelledby="alert-dialog-title"
        aria-describedby="alert-dialog-description"
      >
        <DialogContent>
          <DialogContentText id="alert-dialog-description">
            Depot wirklich löschen? Dies würde auch alle Transaktionen dieses
            Depots löschen!
          </DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button
            onClick={() => setConfirmOpen(false)}
            color="primary"
            autoFocus
          >
            Nein
          </Button>
          <Button onClick={onDelete} color="primary">
            Ja
          </Button>
        </DialogActions>
      </Dialog>
    </>
  );
};

BalanceDialog.propTypes = {
  initialBalance: PropTypes.shape({
    id: PropTypes.number,
  }).isRequired,
  open: PropTypes.bool.isRequired,
  onClose: PropTypes.func.isRequired,
  deleteBalance: PropTypes.func.isRequired,
  updateBalance: PropTypes.func.isRequired,
  createBalance: PropTypes.func.isRequired,
};

const mapDispatchToProps = dispatch => ({
  deleteBalance: id => dispatch(deleteBalanceAction(id)),
  createBalance: t => dispatch(createBalanceAction(t)),
  updateBalance: t => dispatch(updateBalanceAction(t)),
});

export default connect(null, mapDispatchToProps)(BalanceDialog);
