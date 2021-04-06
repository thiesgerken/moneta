import React from 'react';
import PropTypes from 'prop-types';
import { makeStyles } from '@material-ui/core';

const useStyles = makeStyles(theme => ({
  colorRectangle: {
    backgroundColor: theme.palette.primary.main,
    width: '1em',
    height: '1em',
    transform: 'translate(0%, 10%)',
    borderRadius: '20%',
    display: 'inline-block',
    border: '1px solid rgba(0, 0, 0, 0.1)',
    verticalAlign: 'baseline',
    boxShadow:
      '0 2px 3px 0 rgba(0, 0, 0, 0.2), 0 4px 5px 0 rgba(0, 0, 0, 0.19)',
  },
}));

const ColorboxLabel = ({ color, children }) => {
  const classes = useStyles();

  return (
    <span style={{ whiteSpace: 'nowrap' }}>
      <div
        className={classes.colorRectangle}
        style={{ backgroundColor: color }}
      />
      &nbsp;{children}
    </span>
  );
};

ColorboxLabel.propTypes = {
  children: PropTypes.oneOfType([
    PropTypes.arrayOf(PropTypes.node),
    PropTypes.node,
  ]).isRequired,
  color: PropTypes.string.isRequired,
};

export default ColorboxLabel;
